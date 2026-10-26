import { useRef, type ReactElement, type ReactNode, type RefObject } from "react";
import {
  Checkbox,
  Field,
  Input,
  Label,
  Link,
  Select,
  ToggleButton,
  Tooltip,
} from "@fluentui/react-components";
import {
  Document20Regular,
  Folder20Regular,
  FolderArrowRight20Regular,
} from "@fluentui/react-icons";
import type { ApplyTo, RenameOptions, TextCase, TimeSource } from "../api";
import { useI18n, type MessageKey } from "../i18n";
import type { Recent } from "../settings";
import { RegexPatterns, ReplacePatterns } from "./PatternHelp";

interface Props {
  options: RenameOptions;
  onChange: (patch: Partial<RenameOptions>) => void;
  regexError: string | null;
  /** The search text matches none of the items. */
  noMatches: boolean;
  /** `null` when suggestions are turned off. */
  recent: Recent | null;
}

const TEXT_CASES: [TextCase, string, MessageKey][] = [
  ["lower", "aa", "caseLower"],
  ["upper", "AA", "caseUpper"],
  ["title", "Aa", "caseTitle"],
  ["capitalize", "Aa Aa", "caseCapitalize"],
];

const REGEX_SYNTAX = /[.*+?^$()[\]{}|\\]/;
const PATTERN_BODY = String.raw`\{(?:(?:start|increment|padding)=[^}]*|rstring(?:alnum|alpha|digit)=\d*|ruuidv4)?\}`;
const MISSING_DOLLAR = new RegExp(String.raw`(?<!\$)(${PATTERN_BODY})`);
const COUNTER = /\$\{(?:(?:start|increment|padding)=[^}]*)?\}/;
const RANDOM = /\$\{(?:rstring(?:alnum|alpha|digit)=\d+|ruuidv4)\}/;

interface Hint {
  text: MessageKey;
  action: MessageKey;
  fix: Partial<RenameOptions>;
}

/** Common reasons why the replacement doesn't do what was intended. */
function replaceHint(o: RenameOptions): Hint | null {
  if (MISSING_DOLLAR.test(o.replace)) {
    const fixed = o.replace.replace(new RegExp(MISSING_DOLLAR, "g"), "$$$1");
    return { text: "hintAddDollar", action: "fixAddDollar", fix: { replace: fixed } };
  }
  if (!o.enumerate && COUNTER.test(o.replace)) {
    return { text: "hintEnumerate", action: "fixTurnOn", fix: { enumerate: true } };
  }
  if (!o.random && RANDOM.test(o.replace)) {
    return { text: "hintRandom", action: "fixTurnOn", fix: { random: true } };
  }
  return null;
}

function HintMessage({ text, action, onClick }: { text: string; action: string; onClick: () => void }) {
  return (
    <span>
      {text}{" "}
      <Link as="button" inline onClick={onClick}>
        {action}
      </Link>
    </span>
  );
}

function Section({ label, children }: { label: string; children: ReactNode }) {
  return (
    <div className="section" role="group" aria-label={label}>
      <Label weight="semibold">{label}</Label>
      {children}
    </div>
  );
}

export function RenamePanel({ options, onChange, regexError, noMatches, recent }: Props) {
  const { t } = useI18n();
  const searchRef = useRef<HTMLInputElement>(null);
  const replaceRef = useRef<HTMLInputElement>(null);
  const suggestRegex = !options.useRegex && noMatches && REGEX_SYNTAX.test(options.search);
  const hint = replaceHint(options);

  const insertAtCursor =
    (field: "search" | "replace", ref: RefObject<HTMLInputElement | null>) =>
    (text: string, enable?: Partial<RenameOptions>) => {
      const el = ref.current;
      const value = options[field];
      const start = el?.selectionStart ?? value.length;
      const end = el?.selectionEnd ?? start;
      onChange({ ...enable, [field]: value.slice(0, start) + text + value.slice(end) });
      requestAnimationFrame(() => {
        el?.focus();
        el?.setSelectionRange(start + text.length, start + text.length);
      });
    };

  const includeToggle = (
    key: "includeFiles" | "includeFolders" | "includeSubfolders",
    icon: ReactElement,
    label: MessageKey,
    hint: MessageKey,
  ) => (
    <Tooltip content={t(hint)} relationship="description">
      <ToggleButton
        size="small"
        icon={icon}
        checked={options[key]}
        onClick={() => onChange({ [key]: !options[key] })}
      >
        {t(label)}
      </ToggleButton>
    </Tooltip>
  );

  return (
    <div className="panel">
      <Field
        label={t("searchFor")}
        validationState={regexError ? "error" : suggestRegex ? "warning" : "none"}
        validationMessage={
          regexError ? (
            t("regexInvalid", { message: regexError })
          ) : suggestRegex ? (
            <HintMessage
              text={t("hintUseRegex", { search: options.search })}
              action={t("fixUseRegex")}
              onClick={() => onChange({ useRegex: true })}
            />
          ) : undefined
        }
      >
        <Input
          ref={searchRef}
          value={options.search}
          onChange={(_, d) => onChange({ search: d.value })}
          placeholder={t("searchPlaceholder")}
          list={recent ? "recent-search" : undefined}
          spellCheck={false}
          autoComplete="off"
          contentAfter={<RegexPatterns options={options} onInsert={insertAtCursor("search", searchRef)} />}
        />
      </Field>
      <Field
        label={t("replaceWith")}
        validationState={hint ? "warning" : "none"}
        validationMessage={
          hint ? <HintMessage text={t(hint.text)} action={t(hint.action)} onClick={() => onChange(hint.fix)} /> : undefined
        }
      >
        <Input
          ref={replaceRef}
          value={options.replace}
          onChange={(_, d) => onChange({ replace: d.value })}
          placeholder={t("replacePlaceholder")}
          list={recent ? "recent-replace" : undefined}
          spellCheck={false}
          autoComplete="off"
          contentAfter={<ReplacePatterns options={options} onInsert={insertAtCursor("replace", replaceRef)} />}
        />
      </Field>
      {recent && (
        <>
          <datalist id="recent-search">
            {recent.search.map((v) => (
              <option key={v} value={v} />
            ))}
          </datalist>
          <datalist id="recent-replace">
            {recent.replace.map((v) => (
              <option key={v} value={v} />
            ))}
          </datalist>
        </>
      )}

      <div className="checks">
        <Checkbox
          checked={options.useRegex}
          onChange={(_, d) => onChange({ useRegex: !!d.checked })}
          label={t("useRegex")}
        />
        <Checkbox
          checked={options.matchAll}
          onChange={(_, d) => onChange({ matchAll: !!d.checked })}
          label={t("matchAll")}
        />
        <Checkbox
          checked={options.caseSensitive}
          onChange={(_, d) => onChange({ caseSensitive: !!d.checked })}
          label={t("caseSensitive")}
        />
      </div>

      <Field label={t("applyTo")}>
        <Select
          value={options.applyTo}
          onChange={(_, d) => onChange({ applyTo: d.value as ApplyTo })}
        >
          <option value="name">{t("applyToName")}</option>
          <option value="extension">{t("applyToExtension")}</option>
          <option value="both">{t("applyToBoth")}</option>
        </Select>
      </Field>

      <Section label={t("include")}>
        <div className="toggle-row">
          {includeToggle("includeFiles", <Document20Regular />, "includeFiles", "includeFilesHint")}
          {includeToggle("includeFolders", <Folder20Regular />, "includeFolders", "includeFoldersHint")}
          {includeToggle(
            "includeSubfolders",
            <FolderArrowRight20Regular />,
            "includeSubfolders",
            "includeSubfoldersHint",
          )}
        </div>
      </Section>

      <Section label={t("textFormatting")}>
        <div className="toggle-row">
          {TEXT_CASES.map(([value, glyph, label]) => (
            <Tooltip key={value} content={t(label)} relationship="label">
              <ToggleButton
                size="small"
                className="case-button"
                checked={options.textCase === value}
                onClick={() => onChange({ textCase: options.textCase === value ? "none" : value })}
              >
                {glyph}
              </ToggleButton>
            </Tooltip>
          ))}
        </div>
      </Section>

      <Section label={t("replaceOptions")}>
        <div className="checks">
          <Checkbox
            checked={options.enumerate}
            onChange={(_, d) => onChange({ enumerate: !!d.checked })}
            label={t("enumerate")}
          />
          <Checkbox
            checked={options.random}
            onChange={(_, d) => onChange({ random: !!d.checked })}
            label={t("random")}
          />
        </div>
      </Section>

      <Field label={t("dateSource")}>
        <Select
          value={options.timeSource}
          onChange={(_, d) => onChange({ timeSource: d.value as TimeSource })}
        >
          <option value="created">{t("dateCreated")}</option>
          <option value="modified">{t("dateModified")}</option>
        </Select>
      </Field>
    </div>
  );
}
