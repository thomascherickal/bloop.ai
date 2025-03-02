import React, { useContext, useEffect, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import { Trans, useTranslation } from 'react-i18next';
// eslint-disable-next-line import/no-duplicates
import { format } from 'date-fns';
// eslint-disable-next-line import/no-duplicates
import { ja } from 'date-fns/locale';
import {
  Checkmark,
  List,
  MagnifyTool,
  PenUnderline,
  PointClick,
  QuillIcon,
  Sparkles,
  WrenchAndScrewdriver,
} from '../../../icons';
import { DeviceContext } from '../../../context/deviceContext';
import { ChatLoadingStep, ChatMessageAuthor } from '../../../types/general';
import { ChatContext } from '../../../context/chatContext';
import Button from '../../Button';
import { FileModalContext } from '../../../context/fileModalContext';
import { LocaleContext } from '../../../context/localeContext';
import MessageFeedback from './MessageFeedback';
import FileChip from './FileChip';
import SummaryCardsArticle from './SummaryCards/SummaryCardsArticle';

type Props = {
  author: ChatMessageAuthor;
  message?: string;
  error?: string;
  threadId: string;
  queryId: string;
  repoRef: string;
  repoName: string;
  responseTimestamp: string | null;
  isHistory?: boolean;
  showInlineFeedback: boolean;
  scrollToBottom?: () => void;
  isLoading?: boolean;
  loadingSteps?: ChatLoadingStep[];
  results?: string;
  i: number;
  onMessageEdit: (queryId: string, i: number) => void;
  explainedFile?: string;
};

const ConversationMessage = ({
  author,
  message,
  error,
  isHistory,
  showInlineFeedback,
  threadId,
  queryId,
  repoRef,
  scrollToBottom,
  isLoading,
  loadingSteps,
  results,
  i,
  repoName,
  onMessageEdit,
  responseTimestamp,
  explainedFile,
}: Props) => {
  const { t } = useTranslation();
  const [isLoadingStepsShown, setLoadingStepsShown] = useState(false);
  const { envConfig } = useContext(DeviceContext);
  const { setChatOpen } = useContext(ChatContext.Setters);
  const { openFileModal } = useContext(FileModalContext);
  const { locale } = useContext(LocaleContext);

  useEffect(() => {
    setChatOpen(true);
  }, []);

  return (
    <div className="flex flex-col">
      {author === ChatMessageAuthor.Server && !!loadingSteps?.length && (
        <div
          className={`${
            isLoadingStepsShown ? 'mb-3' : ''
          } flex flex-col gap-3 px-4 overflow-hidden transition-all duration-200 ease-linear`}
          style={{
            maxHeight: isLoadingStepsShown ? loadingSteps.length * 36 : 0,
          }}
        >
          {loadingSteps.map((s, i) => (
            <div
              className="flex gap-2 caption text-label-base items-center"
              key={i}
            >
              {s.type === 'proc' ? <PointClick /> : <MagnifyTool />}
              <span>{s.type === 'proc' ? t('Reading ') : s.displayText}</span>
              {s.type === 'proc' ? (
                <FileChip
                  onClick={() => openFileModal(s.path)}
                  fileName={s.path.split('/').pop() || ''}
                  filePath={s.path || ''}
                />
              ) : null}
            </div>
          ))}
        </div>
      )}
      {author === ChatMessageAuthor.Server && (
        <div className="flex gap-2 px-4 items-center">
          {!isLoading ? (
            <div className="text-bg-success-hover h-5">
              <Checkmark />
            </div>
          ) : (
            <div className="text-label-base h-5">
              <MagnifyTool />
            </div>
          )}
          <div className="caption text-label-base flex-1 flex gap-2 items-center">
            <p>{isLoading ? t('Generating response...') : t('Answer Ready')}</p>
            <Button
              size="tiny"
              variant={isLoadingStepsShown ? 'tertiary-active' : 'tertiary'}
              onlyIcon
              title={t(`${isLoadingStepsShown ? 'Hide' : 'Show'} search steps`)}
              onClick={() => setLoadingStepsShown((prev) => !prev)}
            >
              <List />
            </Button>
          </div>
          {!isLoading && !!responseTimestamp && (
            <div className="justify-self-end caption text-label-muted">
              {format(
                new Date(responseTimestamp),
                'hh:mm aa',
                locale === 'ja' ? { locale: ja } : undefined,
              )}
            </div>
          )}
        </div>
      )}
      {message ? (
        <>
          {!isLoading && !!results?.length ? (
            <div className="mt-3 select-none cursor-default group-summary">
              {!!results ? (
                <SummaryCardsArticle
                  article={results}
                  threadId={threadId}
                  i={i}
                  explainedFile={explainedFile}
                />
              ) : null}
            </div>
          ) : null}
          <div
            className={`relative bg-chat-bg-shade mt-3 flex items-start p-4 gap-3 border border-chat-bg-divider rounded-lg`}
          >
            <div className="relative">
              <div className="w-6 h-6 rounded-full bg-chat-bg-border overflow-hidden flex items-center justify-center select-none">
                {author === ChatMessageAuthor.User ? (
                  <img
                    src={envConfig.github_user?.avatar_url}
                    alt={t('avatar')}
                  />
                ) : (
                  <div className="w-3 h-3">
                    <Sparkles raw />
                  </div>
                )}
              </div>
              {author === ChatMessageAuthor.User && (
                <div className="absolute -bottom-1 -right-1 w-4 h-3 bg-chat-bg-border box-content border-2 border-chat-bg-shade text-label-title rounded-full flex items-center justify-center">
                  <div className="w-1.5 h-2">
                    <QuillIcon raw />
                  </div>
                </div>
              )}
            </div>
            {message && (
              <pre className="body-s text-label-title whitespace-pre-wrap break-word markdown w-full">
                {author === ChatMessageAuthor.Server ? (
                  <ReactMarkdown>{message}</ReactMarkdown>
                ) : (
                  <>
                    <span>{message}</span>
                    {!isHistory && !!queryId && (
                      <div className="absolute bottom-1 right-1">
                        <Button
                          size="tiny"
                          variant="tertiary"
                          onlyIcon
                          title={t('Edit')}
                          onClick={() => onMessageEdit(queryId, i)}
                        >
                          <PenUnderline />
                        </Button>
                      </div>
                    )}
                  </>
                )}
              </pre>
            )}
          </div>
          <MessageFeedback
            showInlineFeedback={showInlineFeedback}
            isHistory={isHistory}
            threadId={threadId}
            queryId={queryId}
            repoRef={repoRef}
            error={!!error}
            scrollToBottom={scrollToBottom}
          />
        </>
      ) : error ? (
        <div className="flex items-start gap-3 text-bg-danger p-4 mt-3 rounded-lg bg-[linear-gradient(90deg,rgba(251,113,133,0.08)_0%,rgba(231,139,152,0.08)_33.18%,rgba(191,191,191,0.08)_100%)]">
          <WrenchAndScrewdriver />
          <div className="flex flex-col gap-1">
            <p className="body-s text-label-title">
              <Trans>Something went wrong</Trans>
            </p>
            <p className="body-s text-label-base">{error}</p>
          </div>
        </div>
      ) : null}
    </div>
  );
};

export default ConversationMessage;
