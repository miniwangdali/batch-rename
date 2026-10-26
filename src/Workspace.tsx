import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Button,
  MessageBar,
  MessageBarActions,
  MessageBarBody,
  MessageBarTitle,
  Spinner,
  Text,
  Tooltip,
} from "@fluentui/react-components";
import {
  ArrowUndo20Regular,
  Delete20Regular,
  Dismiss20Regular,
  DocumentAdd20Regular,
  DocumentMultiple20Regular,
  FolderAdd20Regular,
  Rename20Regular,
  Settings20Regular,
} from "@fluentui/react-icons";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebview } from "@tauri-apps/api/webview";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { open } from "@tauri-apps/plugin-dialog";
import {
  api,
  type Item,
  type ItemsResponse,
  type Preview,
  type RenameOptions,
  type RenameResponse,
} from "./api";
import { PreviewList } from "./components/PreviewList";
import { RenamePanel } from "./components/RenamePanel";
import { SettingsDialog } from "./components/SettingsDialog";
import { dateNames, useI18n } from "./i18n";
import {
  initialOptions,
  loadRecent,
  pushRecent,
  saveOptions,
  saveRecent,
  type Settings,
} from "./settings";

const MAX_ITEMS = 100_000;
const PREVIEW_DELAY_MS = 80;

interface Notice {
  intent: "success" | "warning" | "error" | "info";
  title: string;
  body?: string;
}

interface Props {
  settings: Settings;
  onSettingsChange: (settings: Settings) => void;
}

export function Workspace({ settings, onSettingsChange }: Props) {
  const { t, locale } = useI18n();
  const [items, setItems] = useState<Item[]>([]);
  const [deselected, setDeselected] = useState<Set<number>>(() => new Set());
  const [recent, setRecent] = useState(loadRecent);
  const [options, setOptions] = useState<RenameOptions>(() => initialOptions(settings, recent));
  const [preview, setPreview] = useState<Preview | null>(null);
  const [previewPending, setPreviewPending] = useState(false);
  const [regexError, setRegexError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [canUndo, setCanUndo] = useState(false);
  const [onlyRenamed, setOnlyRenamed] = useState(false);
  const [dragOver, setDragOver] = useState(false);
  const [notice, setNotice] = useState<Notice | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);

  const names = useMemo(() => dateNames(locale), [locale]);
  const entries = useMemo(
    () => new Map((preview?.entries ?? []).map((e) => [e.id, e])),
    [preview],
  );

  useEffect(() => {
    getCurrentWindow().setTitle(t("appTitle")).catch(() => {});
  }, [t]);

  useEffect(() => {
    if (settings.restoreOptions) saveOptions(options);
  }, [options, settings.restoreOptions]);

  useEffect(() => {
    if (notice?.intent !== "success") return;
    const timer = setTimeout(() => setNotice(null), 5000);
    return () => clearTimeout(timer);
  }, [notice]);

  const updateOptions = useCallback(
    (patch: Partial<RenameOptions>) => setOptions((o) => ({ ...o, ...patch })),
    [],
  );

  const showError = (e: unknown) => setNotice({ intent: "error", title: String(e) });

  const handleItems = (res: ItemsResponse) => {
    setItems(res.items);
    const warnings = [];
    if (res.report.missing.length) warnings.push(t("missingPaths", { count: res.report.missing.length }));
    if (res.report.truncated) warnings.push(t("truncated", { max: MAX_ITEMS.toLocaleString(locale) }));
    if (warnings.length) setNotice({ intent: "warning", title: warnings.join(" ") });
  };
  // Event listeners below are registered once; this ref gives them the latest handler.
  const handleItemsRef = useRef(handleItems);
  handleItemsRef.current = handleItems;

  const addPaths = useCallback(async (paths: string[]) => {
    if (!paths.length) return;
    try {
      handleItemsRef.current(await api.addPaths(paths));
    } catch (e) {
      showError(e);
    }
  }, []);

  // Paths passed on the command line, e.g. from the File Explorer context menu. Later launches
  // are forwarded to this window by the single-instance plugin.
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const pull = () => api.addPending().then((res) => handleItemsRef.current(res), showError);
    pull();
    const unlisten = listen("pending-paths", () => {
      clearTimeout(timer);
      timer = setTimeout(pull, 50);
    });
    return () => {
      clearTimeout(timer);
      unlisten.then((f) => f());
    };
  }, []);

  useEffect(() => {
    const unlisten = getCurrentWebview().onDragDropEvent(({ payload }) => {
      if (payload.type === "enter" || payload.type === "over") {
        setDragOver(true);
      } else {
        setDragOver(false);
        if (payload.type === "drop") addPaths(payload.paths);
      }
    });
    return () => {
      unlisten.then((f) => f());
    };
  }, [addPaths]);

  // Live preview, debounced; stale responses are ignored.
  const previewSeq = useRef(0);
  useEffect(() => {
    const seq = ++previewSeq.current;
    if (!items.length) {
      setPreview(null);
      setRegexError(null);
      setPreviewPending(false);
      return;
    }
    setPreviewPending(true);
    const timer = setTimeout(() => {
      api.preview({ ...options, dateNames: names }, [...deselected]).then(
        (p) => {
          if (seq !== previewSeq.current) return;
          setPreview(p);
          setRegexError(null);
          setPreviewPending(false);
        },
        (e) => {
          if (seq !== previewSeq.current) return;
          setPreview(null);
          setRegexError(String(e));
          setPreviewPending(false);
        },
      );
    }, PREVIEW_DELAY_MS);
    return () => clearTimeout(timer);
  }, [items, options, deselected, names]);

  const toggle = useCallback(
    (id: number) =>
      setDeselected((prev) => {
        const next = new Set(prev);
        if (!next.delete(id)) next.add(id);
        return next;
      }),
    [],
  );

  const toggleAll = useCallback(
    () =>
      setDeselected((prev) =>
        items.some((i) => prev.has(i.id)) ? new Set() : new Set(items.map((i) => i.id)),
      ),
    [items],
  );

  const pick = async (directory: boolean) => {
    try {
      const selected = await open({
        multiple: true,
        directory,
        title: t(directory ? "addFolder" : "addFiles"),
      });
      if (selected) await addPaths(Array.isArray(selected) ? selected : [selected]);
    } catch (e) {
      showError(e);
    }
  };

  const clear = async () => {
    try {
      await api.clearItems();
      setItems([]);
      setDeselected(new Set());
      setCanUndo(false);
      setNotice(null);
    } catch (e) {
      showError(e);
    }
  };

  const finish = (res: RenameResponse, successKey: "renamedMessage" | "undoneMessage") => {
    setItems(res.items);
    setCanUndo(res.report.canUndo);
    const { renamed, failures } = res.report;
    if (failures.length) {
      const lines = failures.slice(0, 5).map((f) => `${f.name}: ${f.message}`);
      if (failures.length > 5) lines.push("…");
      setNotice({
        intent: "error",
        title: t("failuresTitle", { count: failures.length }),
        body: lines.join("\n"),
      });
      return false;
    }
    setNotice({ intent: "success", title: t(successKey, { count: renamed }) });
    return true;
  };

  const canRename = !!preview && !previewPending && !busy && preview.renamable > 0;

  const doRename = async () => {
    if (!canRename || !preview) return;
    setBusy(true);
    try {
      const res = await api.rename(preview.version);
      const updated = pushRecent(recent, options.search, options.replace, settings.maxRecent);
      setRecent(updated);
      saveRecent(updated);
      if (finish(res, "renamedMessage") && settings.closeAfterRename) {
        await getCurrentWindow().close();
      }
    } catch (e) {
      showError(e);
    } finally {
      setBusy(false);
    }
  };

  const doUndo = async () => {
    setBusy(true);
    try {
      finish(await api.undo(), "undoneMessage");
    } catch (e) {
      showError(e);
    } finally {
      setBusy(false);
    }
  };

  const doRenameRef = useRef(doRename);
  doRenameRef.current = doRename;
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key === "Enter") {
        e.preventDefault();
        doRenameRef.current();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const selectedCount = useMemo(
    () => items.reduce((n, i) => n + (deselected.has(i.id) ? 0 : 1), 0),
    [items, deselected],
  );
  const errorCount = preview?.errors ?? 0;

  return (
    <div className="app">
      <aside className="side">
        <RenamePanel
          options={options}
          onChange={updateOptions}
          regexError={regexError}
          recent={settings.rememberRecent ? recent : null}
        />
      </aside>

      <main className="main">
        <div className="toolbar">
          <Button appearance="subtle" icon={<DocumentAdd20Regular />} onClick={() => pick(false)}>
            {t("addFiles")}
          </Button>
          <Button appearance="subtle" icon={<FolderAdd20Regular />} onClick={() => pick(true)}>
            {t("addFolder")}
          </Button>
          <Button
            appearance="subtle"
            icon={<Delete20Regular />}
            onClick={clear}
            disabled={!items.length || busy}
          >
            {t("clear")}
          </Button>
          <span className="spacer" />
          <Tooltip content={t("settings")} relationship="label">
            <Button appearance="subtle" icon={<Settings20Regular />} onClick={() => setSettingsOpen(true)} />
          </Tooltip>
        </div>

        {notice && (
          <MessageBar intent={notice.intent} layout={notice.body ? "multiline" : "auto"} className="notice">
            <MessageBarBody>
              <MessageBarTitle>{notice.title}</MessageBarTitle>
              {notice.body && <div className="notice-body">{notice.body}</div>}
            </MessageBarBody>
            <MessageBarActions
              containerAction={
                <Button
                  appearance="transparent"
                  aria-label={t("dismiss")}
                  icon={<Dismiss20Regular />}
                  onClick={() => setNotice(null)}
                />
              }
            />
          </MessageBar>
        )}

        {items.length ? (
          <PreviewList
            items={items}
            entries={entries}
            deselected={deselected}
            onToggle={toggle}
            onToggleAll={toggleAll}
            onlyRenamed={onlyRenamed}
            onOnlyRenamedChange={setOnlyRenamed}
          />
        ) : (
          <div className="empty">
            <DocumentMultiple20Regular className="empty-icon" />
            <Text size={500} weight="semibold">
              {t("emptyTitle")}
            </Text>
            <Text className="hint">{t("emptyHint")}</Text>
            <div className="empty-actions">
              <Button icon={<DocumentAdd20Regular />} onClick={() => pick(false)}>
                {t("addFiles")}
              </Button>
              <Button icon={<FolderAdd20Regular />} onClick={() => pick(true)}>
                {t("addFolder")}
              </Button>
            </div>
          </div>
        )}

        <footer className="footer">
          <Text className="status">
            {t("status", {
              count: items.length,
              total: items.length,
              selected: selectedCount,
              renaming: preview?.renamable ?? 0,
            })}
            {errorCount > 0 && (
              <span className="status-errors"> · {t("statusErrors", { count: errorCount })}</span>
            )}
          </Text>
          {(busy || previewPending) && <Spinner size="extra-tiny" />}
          <Button icon={<ArrowUndo20Regular />} disabled={!canUndo || busy} onClick={doUndo}>
            {t("undo")}
          </Button>
          <Button
            appearance="primary"
            icon={<Rename20Regular />}
            disabled={!canRename}
            onClick={doRename}
          >
            {t("rename")}
          </Button>
        </footer>
      </main>

      {dragOver && (
        <div className="drop-overlay">
          <Text size={600} weight="semibold">
            {t("dropToAdd")}
          </Text>
        </div>
      )}

      <SettingsDialog
        open={settingsOpen}
        onOpenChange={setSettingsOpen}
        settings={settings}
        onChange={onSettingsChange}
      />
    </div>
  );
}
