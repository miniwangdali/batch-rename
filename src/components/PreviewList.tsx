import { memo, useMemo, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { Checkbox, ToggleButton, Tooltip } from "@fluentui/react-components";
import {
  Document20Regular,
  ErrorCircle16Regular,
  Filter20Regular,
  Folder20Regular,
} from "@fluentui/react-icons";
import type { ErrorCode, Item, PreviewEntry } from "../api";
import { useI18n, type MessageKey, type Translate } from "../i18n";

const ROW_HEIGHT = 32;
const INDENT = 18;

const ERROR_MESSAGES: Record<ErrorCode, MessageKey> = {
  empty: "errEmpty",
  illegalChars: "errIllegalChars",
  reserved: "errReserved",
  trailing: "errTrailing",
  tooLong: "errTooLong",
  duplicate: "errDuplicate",
  exists: "errExists",
  regex: "errRegex",
};

interface Props {
  items: Item[];
  entries: Map<number, PreviewEntry>;
  deselected: Set<number>;
  onToggle: (id: number) => void;
  onToggleAll: () => void;
  onlyRenamed: boolean;
  onOnlyRenamedChange: (value: boolean) => void;
}

/** Builds full paths lazily; only top-level items carry their directory. */
function usePathOf(items: Item[]) {
  return useMemo(() => {
    const byId = new Map(items.map((i) => [i.id, i]));
    const cache = new Map<number, string>();
    const join = (dir: string, name: string) => {
      const sep = dir.includes("\\") ? "\\" : "/";
      return dir.endsWith(sep) ? dir + name : dir + sep + name;
    };
    const pathOf = (item: Item): string => {
      let path = cache.get(item.id);
      if (path === undefined) {
        const parent = item.parent === undefined ? undefined : byId.get(item.parent);
        path = join(parent ? pathOf(parent) : (item.dir ?? ""), item.name);
        cache.set(item.id, path);
      }
      return path;
    };
    return pathOf;
  }, [items]);
}

interface RowProps {
  item: Item;
  entry: PreviewEntry | undefined;
  checked: boolean;
  top: number;
  path: string;
  onToggle: (id: number) => void;
  t: Translate;
}

const Row = memo(function Row({ item, entry, checked, top, path, onToggle, t }: RowProps) {
  const indent = { paddingInlineStart: item.depth * INDENT };
  const error = entry?.error
    ? t(ERROR_MESSAGES[entry.error]) + (entry.detail ? `: ${entry.detail}` : "")
    : undefined;
  return (
    <div
      className={`row${checked ? "" : " off"}`}
      style={{ transform: `translateY(${top}px)` }}
      role="row"
    >
      <div className="cell-check">
        <Checkbox checked={checked} onChange={() => onToggle(item.id)} aria-label={item.name} />
      </div>
      <div className="cell" style={indent} title={path}>
        {item.isDir ? <Folder20Regular /> : <Document20Regular />}
        <span className="text">{item.name}</span>
      </div>
      <div
        className={`cell renamed${error ? " error" : ""}`}
        style={indent}
        title={error ?? entry?.newName}
      >
        {entry && (
          <>
            {error && <ErrorCircle16Regular />}
            <span className="text">{entry.newName}</span>
          </>
        )}
      </div>
    </div>
  );
});

export function PreviewList({
  items,
  entries,
  deselected,
  onToggle,
  onToggleAll,
  onlyRenamed,
  onOnlyRenamedChange,
}: Props) {
  const { t } = useI18n();
  const scrollRef = useRef<HTMLDivElement>(null);
  const pathOf = usePathOf(items);
  const rows = useMemo(
    () => (onlyRenamed ? items.filter((i) => entries.has(i.id)) : items),
    [items, entries, onlyRenamed],
  );
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => ROW_HEIGHT,
    overscan: 10,
  });

  const deselectedCount = useMemo(
    () => items.reduce((n, i) => n + (deselected.has(i.id) ? 1 : 0), 0),
    [items, deselected],
  );
  const allState = deselectedCount === 0 ? true : deselectedCount === items.length ? false : "mixed";

  return (
    <div className="list" role="table">
      <div className="list-header" role="row">
        <div className="cell-check">
          <Checkbox checked={allState} onChange={onToggleAll} aria-label={t("selectAll")} />
        </div>
        <div className="header-cell">
          {t("original")}
          <span className="count">{items.length}</span>
        </div>
        <div className="header-cell">
          {t("renamed")}
          <span className="count">{entries.size}</span>
          <span className="spacer" />
          <Tooltip content={t("onlyRenamed")} relationship="label">
            <ToggleButton
              size="small"
              appearance="subtle"
              icon={<Filter20Regular />}
              checked={onlyRenamed}
              onClick={() => onOnlyRenamedChange(!onlyRenamed)}
            />
          </Tooltip>
        </div>
      </div>
      <div className="list-body" ref={scrollRef}>
        <div style={{ height: virtualizer.getTotalSize(), position: "relative" }}>
          {virtualizer.getVirtualItems().map((v) => {
            const item = rows[v.index];
            return (
              <Row
                key={item.id}
                item={item}
                entry={entries.get(item.id)}
                checked={!deselected.has(item.id)}
                top={v.start}
                path={pathOf(item)}
                onToggle={onToggle}
                t={t}
              />
            );
          })}
        </div>
      </div>
    </div>
  );
}
