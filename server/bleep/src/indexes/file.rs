use std::{
    collections::HashSet,
    ops::Not,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::{Context, Result};
use async_trait::async_trait;
use dashmap::mapref::entry::Entry;
use tantivy::{
    collector::TopDocs,
    doc,
    query::{BooleanQuery, QueryParser, TermQuery},
    schema::{
        BytesOptions, Field, IndexRecordOption, Schema, Term, TextFieldIndexing, TextOptions, FAST,
        STORED, STRING,
    },
    IndexWriter,
};
use tracing::{debug, info, trace, warn};

use super::{
    reader::{ContentDocument, ContentReader},
    DocumentRead, Indexable, Indexer,
};
use crate::{
    intelligence::TreeSitterFile,
    state::{FileCache, RepoHeadInfo, RepoRef, Repository},
    symbol::SymbolLocations,
    Configuration,
};

struct Workload<'a> {
    file_disk_path: PathBuf,
    repo_disk_path: &'a Path,
    repo_ref: String,
    repo_name: &'a str,
    repo_info: &'a RepoHeadInfo,
    cache: &'a FileCache,
}

#[derive(Clone)]
pub struct File {
    config: Arc<Configuration>,
    schema: Schema,

    // Path to the indexed file on disk
    pub file_disk_path: Field,
    // Path to the root of the repo on disk
    pub repo_disk_path: Field,
    // Path to the file, relative to the repo root
    pub relative_path: Field,

    // Unique repo identifier, of the form:
    //  local: local//path/to/repo
    // github: github.com/org/repo
    pub repo_ref: Field,

    // Indexed repo name, of the form:
    //  local: repo
    // github: github.com/org/repo
    pub repo_name: Field,

    pub content: Field,
    pub line_end_indices: Field,

    // a flat list of every symbol's text, for searching, e.g.: ["File", "Repo", "worker"]
    pub symbols: Field,
    pub symbol_locations: Field,

    // fast fields for scoring
    pub lang: Field,
    pub avg_line_length: Field,
    pub last_commit_unix_seconds: Field,
}

impl File {
    pub fn new(config: Arc<Configuration>) -> Self {
        let mut builder = tantivy::schema::SchemaBuilder::new();
        let trigram = TextOptions::default().set_stored().set_indexing_options(
            TextFieldIndexing::default()
                .set_tokenizer("default")
                .set_index_option(IndexRecordOption::WithFreqsAndPositions),
        );

        let file_disk_path = builder.add_text_field("file_disk_path", STRING);
        let repo_disk_path = builder.add_text_field("repo_disk_path", STRING);
        let repo_ref = builder.add_text_field("repo_ref", STRING | STORED);
        let repo_name = builder.add_text_field("repo_name", trigram.clone());
        let relative_path = builder.add_text_field("relative_path", trigram.clone());

        let content = builder.add_text_field("content", trigram.clone());
        let line_end_indices =
            builder.add_bytes_field("line_end_indices", BytesOptions::default().set_stored());

        let symbols = builder.add_text_field("symbols", trigram);
        let symbol_locations =
            builder.add_bytes_field("symbol_locations", BytesOptions::default().set_stored());

        let lang = builder.add_bytes_field(
            "lang",
            BytesOptions::default().set_stored().set_indexed() | FAST,
        );
        let avg_line_length = builder.add_f64_field("line_length", FAST);
        let last_commit_unix_seconds = builder.add_u64_field("last_commit_unix_seconds", FAST);

        Self {
            file_disk_path,
            repo_disk_path,
            relative_path,
            repo_ref,
            repo_name,
            content,
            line_end_indices,
            symbols,
            symbol_locations,
            lang,
            avg_line_length,
            last_commit_unix_seconds,
            schema: builder.build(),
            config,
        }
    }
}

#[async_trait]
impl Indexable for File {
    fn index_repository(
        &self,
        reporef: &RepoRef,
        repo: &Repository,
        repo_info: &RepoHeadInfo,
        writer: &IndexWriter,
    ) -> Result<()> {
        let file_cache = repo.open_file_cache(&self.config.index_dir)?;
        let repo_name = reporef.indexed_name();

        // note: this WILL observe .gitignore files for the respective repos.
        let walker = repo
            .open_walker()
            .filter_map(|entry| match entry {
                Ok(de) => match de.file_type() {
                    Some(ft) if ft.is_file() => Some(dunce::canonicalize(de.into_path()).unwrap()),
                    _ => None,
                },
                Err(err) => {
                    warn!(%err, "access failure; skipping");
                    None
                }
            })
            .collect::<Vec<PathBuf>>();

        let start = std::time::Instant::now();

        use rayon::prelude::*;
        walker.par_iter().for_each(|file_disk_path| {
            let workload = Workload {
                file_disk_path: file_disk_path.clone(),
                repo_disk_path: &repo.disk_path,
                repo_ref: reporef.to_string(),
                repo_name: &repo_name,
                cache: &file_cache,
                repo_info,
            };

            debug!(?file_disk_path, "queueing file");
            if let Err(err) = worker(self.clone(), workload, writer) {
                warn!(%err, ?file_disk_path, "indexing failed; skipping");
            }
        });

        info!(?repo.disk_path, "file indexing finished, took {:?}", start.elapsed());

        file_cache.retain(|k, v| {
            if v.fresh.not() {
                writer.delete_term(Term::from_field_text(
                    self.file_disk_path,
                    &k.to_string_lossy(),
                ));
            }

            v.fresh
        });

        repo.save_file_cache(&self.config.index_dir, file_cache)?;
        Ok(())
    }

    fn delete_by_repo(&self, writer: &IndexWriter, repo: &Repository) {
        writer.delete_term(Term::from_field_text(
            self.repo_disk_path,
            &repo.disk_path.to_string_lossy(),
        ));
    }

    fn schema(&self) -> Schema {
        self.schema.clone()
    }
}

impl Indexer<File> {
    pub async fn file_body(&self, file_disk_path: &str) -> Result<String> {
        // Mostly taken from `by_path`, below.
        //
        // TODO: This can be unified with `by_path` below, but we first need to decide on a unified
        // path referencing API throughout the webserver.

        let reader = self.reader.read().await;
        let searcher = reader.searcher();

        let query = TermQuery::new(
            Term::from_field_text(self.source.file_disk_path, file_disk_path),
            IndexRecordOption::Basic,
        );

        let collector = TopDocs::with_limit(1);
        let search_results = searcher
            .search(&query, &collector)
            .context("failed to search index")?;

        match search_results.as_slice() {
            [] => Err(anyhow::Error::msg("no path found")),
            [(_, doc_addr)] => Ok(searcher
                .doc(*doc_addr)
                .context("failed to get document by address")?
                .get_first(self.source.content)
                .context("content field was missing")?
                .as_text()
                .context("content field did not contain text")?
                .to_owned()),
            _ => {
                warn!("TopDocs is not limited to 1 and index contains duplicates");
                Err(anyhow::Error::msg("multiple paths returned"))
            }
        }
    }

    pub async fn by_path(
        &self,
        repo_ref: &RepoRef,
        relative_path: &str,
    ) -> Result<ContentDocument> {
        let reader = self.reader.read().await;
        let searcher = reader.searcher();

        let file_index = searcher.index();
        let file_source = &self.source;

        // query the `relative_path` field of the `File` index, using tantivy's query language
        //
        // XXX: can we use the bloop query language here instead?
        let query_parser = QueryParser::for_index(
            file_index,
            vec![self.source.repo_disk_path, self.source.relative_path],
        );
        let query = query_parser
            .parse_query(&format!(
                "repo_ref:\"{}\" AND relative_path:\"{}\"",
                repo_ref, relative_path
            ))
            .expect("failed to parse tantivy query");

        let collector = TopDocs::with_limit(1);
        let search_results = searcher
            .search(&query, &collector)
            .expect("failed to search index");

        match search_results.as_slice() {
            // no paths matched, the input path was not well formed
            [] => Err(anyhow::Error::msg("no path found")),

            // exactly one path, good
            [(_, doc_addr)] => {
                let retrieved_doc = searcher
                    .doc(*doc_addr)
                    .expect("failed to get document by address");
                Ok(ContentReader.read_document(file_source, retrieved_doc))
            }

            // more than one path matched, this can occur when top docs is no
            // longer limited to 1 and the index contains dupes
            _ => {
                warn!("TopDocs is not limited to 1 and index contains duplicates");
                Err(anyhow::Error::msg("multiple paths returned"))
            }
        }
    }

    // Produce all files in a repo
    //
    // TODO: Look at this again when:
    //  - directory retrieval is ready
    //  - unified referencing is ready
    pub async fn by_repo(&self, repo_ref: &RepoRef, lang: Option<&str>) -> Vec<ContentDocument> {
        let reader = self.reader.read().await;
        let searcher = reader.searcher();

        // repo query
        let path_query = Box::new(TermQuery::new(
            Term::from_field_text(self.source.repo_ref, &repo_ref.to_string()),
            IndexRecordOption::Basic,
        ));

        // if file has a recognised language, constrain by files of the same lang
        let query = match lang {
            Some(l) => BooleanQuery::intersection(vec![
                path_query,
                // language query
                Box::new(TermQuery::new(
                    Term::from_field_bytes(self.source.lang, l.to_ascii_lowercase().as_bytes()),
                    IndexRecordOption::Basic,
                )),
            ]),
            None => BooleanQuery::intersection(vec![path_query]),
        };

        let collector = TopDocs::with_limit(100);
        searcher
            .search(&query, &collector)
            .expect("failed to search index")
            .into_iter()
            .map(|(_, doc_addr)| {
                let retrieved_doc = searcher
                    .doc(doc_addr)
                    .expect("failed to get document by address");
                ContentReader.read_document(&self.source, retrieved_doc)
            })
            .collect()
    }
}

fn worker(schema: File, workload: Workload<'_>, writer: &IndexWriter) -> Result<()> {
    let Workload {
        file_disk_path,
        repo_ref,
        repo_disk_path,
        repo_name,
        repo_info,
        cache,
    } = workload;

    let mut buffer = match std::fs::read_to_string(&file_disk_path) {
        Err(err) => {
            debug!(%err, ?file_disk_path, "read failed; skipping");
            return Ok(());
        }
        Ok(buffer) => buffer,
    };

    let relative_path = file_disk_path.strip_prefix(repo_disk_path)?;
    trace!(?relative_path, "processing file");

    let content_hash = {
        let mut hash = blake3::Hasher::new();
        hash.update(crate::state::SCHEMA_VERSION.as_bytes());
        hash.update(buffer.as_bytes());
        hash.finalize().to_hex().to_string()
    };

    trace!(?relative_path, "adding cache entry");

    match cache.entry(file_disk_path.clone()) {
        Entry::Occupied(mut val) if val.get().value == content_hash => {
            // skip processing if contents are up-to-date in the cache
            val.get_mut().fresh = true;
            return Ok(());
        }
        Entry::Occupied(mut val) => {
            val.insert(content_hash.into());
        }
        Entry::Vacant(val) => {
            val.insert(content_hash.into());
        }
    }
    trace!(?relative_path, "added cache entry");

    let lang_str = repo_info
        .langs
        .path_map
        .get(&file_disk_path)
        .unwrap_or_else(|| {
            warn!("Path not found in language map");
            &Some("")
        })
        .unwrap_or("");

    // calculate symbol locations
    let symbol_locations = {
        // build a syntax aware representation of the file
        let scope_graph = TreeSitterFile::try_build(buffer.as_bytes(), lang_str)
            .and_then(TreeSitterFile::scope_graph);

        match scope_graph {
            // we have a graph, use that
            Ok(graph) => SymbolLocations::TreeSitter(graph),
            // no graph, try ctags instead
            Err(err) => {
                debug!(?err, %lang_str, ?file_disk_path, "failed to build scope graph");
                match repo_info.symbols.get(relative_path) {
                    Some(syms) => SymbolLocations::Ctags(syms.clone()),
                    // no ctags either
                    _ => {
                        debug!(%lang_str, ?file_disk_path, "failed to build tags");
                        SymbolLocations::Empty
                    }
                }
            }
        }
    };

    // flatten the list of symbols into a string with just text
    let symbols = symbol_locations
        .list()
        .iter()
        .map(|sym| buffer[sym.range.start.byte..sym.range.end.byte].to_owned())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join("\n");

    // add an NL if this file is not NL-terminated
    if !buffer.ends_with('\n') {
        buffer += "\n";
    }

    let line_end_indices = buffer
        .match_indices('\n')
        .flat_map(|(i, _)| u32::to_le_bytes(i as u32))
        .collect::<Vec<_>>();

    let lines_avg = buffer.len() as f64 / buffer.lines().count() as f64;
    let last_commit = repo_info.last_commit_unix_secs;

    trace!(?relative_path, "writing document");

    writer.add_document(doc!(
        schema.repo_disk_path => repo_disk_path.to_string_lossy().as_ref(),
        schema.file_disk_path => file_disk_path.to_string_lossy().as_ref(),
        schema.relative_path => relative_path.to_string_lossy().as_ref(),
        schema.repo_ref => repo_ref,
        schema.repo_name => repo_name,
        schema.content => buffer,
        schema.line_end_indices => line_end_indices,
        schema.lang => lang_str.to_ascii_lowercase().as_bytes(),
        schema.avg_line_length => lines_avg,
        schema.last_commit_unix_seconds => last_commit,
        schema.symbol_locations => bincode::serialize(&symbol_locations)?,
        schema.symbols => symbols,
    ))?;

    trace!(?relative_path, "document written");

    Ok(())
}