import React, { useCallback, useEffect, useMemo, useState } from 'react';
import FileIcon from '../../FileIcon';
import Code from '../Code';
import { ResultClick, Snippet, TokenInfoFile } from '../../../types/results';
import Button from '../../Button';
import BreadcrumbsPath from '../../BreadcrumbsPath';

type Props = {
  snippets: Snippet[];
  language: string;
  filePath: string;
  branch: string;
  repoName: string;
  collapsed?: boolean;
  onClick?: ResultClick;
};

const PREVIEW_NUM = 3;

const countHighlights = (snippets: Snippet[]) => {
  return snippets.reduce((acc: number, item) => {
    return acc + (item.highlights?.length || 0);
  }, 0);
};

const CodeBlockSearch = ({
  snippets,
  language,
  filePath,
  collapsed,
  onClick,
  repoName,
}: Props) => {
  const [isExpanded, setExpanded] = useState(false);

  const handleMouseUp = useCallback(() => {
    if (!document.getSelection()?.toString()) {
      onClick?.(repoName, filePath);
    }
  }, [onClick]);

  const totalMatches = useMemo(() => {
    return countHighlights(snippets);
  }, [snippets]);

  const hiddenMatches = useMemo(() => {
    if (snippets.length > PREVIEW_NUM) {
      return countHighlights(snippets.slice(PREVIEW_NUM));
    }
    return 0;
  }, [snippets]);

  return (
    <div className="w-full border border-gray-700 rounded-4">
      <div className="w-full flex justify-between bg-gray-800 p-3 border-b border-gray-700 gap-2 select-none">
        <div className="flex items-center gap-2 max-w-[calc(100%-85px)] w-full">
          <FileIcon filename={filePath} />
          <BreadcrumbsPath path={filePath} repo={repoName} shouldNavigate />
        </div>
        <div className="flex gap-2 items-center text-gray-500 flex-shrink-0">
          {/*<div className="flex items-center gap-2">*/}
          {/*  <Branch />*/}
          {/*  <span className="body-s">{branch}</span>*/}
          {/*</div>*/}
          {/*<span className="text-gray-700 h-3 border-l border-l-gray-700"></span>*/}
          <span className="body-s text-gray-100">
            {totalMatches} match{totalMatches > 1 ? 'es' : ''}
          </span>
        </div>
      </div>

      <div
        className={`bg-gray-900 text-gray-600 text-xs  border-gray-700 ${
          collapsed ? 'py-2' : 'py-4'
        } ${onClick ? 'cursor-pointer' : ''} w-full overflow-auto`}
      >
        <div onMouseUp={handleMouseUp}>
          {(isExpanded ? snippets : snippets.slice(0, PREVIEW_NUM)).map(
            (snippet, index) => (
              <span key={index}>
                <Code
                  lineStart={snippet.lineStart}
                  code={snippet.code}
                  language={language}
                  highlights={snippet.highlights}
                  symbols={snippet.symbols}
                  onlySymbolLines={collapsed}
                />
                {index !== snippets.length - 1 ? (
                  collapsed ? (
                    <span className="w-full border-t border-gray-700 block my-2" />
                  ) : (
                    <pre className={` bg-gray-900 my-0 px-2`}>
                      <table>
                        <tbody>
                          <tr className="token-line">
                            <td
                              className={`${
                                snippet.symbols?.length ? 'w-5' : 'w-0 px-1'
                              }  text-center`}
                            />
                            <td className="text-gray-500 min-w-6 text-right	text-l select-none">
                              ..
                            </td>
                          </tr>
                        </tbody>
                      </table>
                    </pre>
                  )
                ) : (
                  ''
                )}
              </span>
            ),
          )}
        </div>
        {snippets.length > PREVIEW_NUM && (
          <div
            className={`${
              isExpanded ? 'mt-2' : 'mt-[-38px] pt-6'
            } mb-1 relative flex justify-center align-center bg-gradient-to-b from-transparent via-gray-900/90 to-gray-900`}
          >
            <Button
              variant="secondary"
              size="small"
              onClick={(e) => {
                e.stopPropagation();
                setExpanded((prev) => !prev);
              }}
            >
              {isExpanded
                ? 'Show less'
                : `Show ${hiddenMatches} more match${
                    hiddenMatches > 1 ? 'es' : ''
                  }`}
            </Button>
          </div>
        )}
      </div>
    </div>
  );
};
export default CodeBlockSearch;