import { Fragment, useState, type ReactElement } from "react";
import {
  Button,
  Popover,
  PopoverSurface,
  PopoverTrigger,
  Text,
  Tooltip,
} from "@fluentui/react-components";
import { BracesVariable20Regular, QuestionCircle20Regular } from "@fluentui/react-icons";
import type { RenameOptions } from "../api";
import { useI18n, type MessageKey } from "../i18n";

type Requirement = "enumerate" | "random" | "useRegex";

interface PatternGroup {
  title: MessageKey;
  requires?: Requirement;
  hint?: MessageKey;
  patterns: [code: string, description: MessageKey][];
}

const REPLACE_GROUPS: PatternGroup[] = [
  {
    title: "groupCounters",
    requires: "enumerate",
    hint: "needsEnumerate",
    patterns: [
      ["${}", "tokCounter"],
      ["${start=1}", "tokStart"],
      ["${increment=2}", "tokIncrement"],
      ["${padding=3}", "tokPadding"],
      ["${start=1,increment=1,padding=3}", "tokCombined"],
    ],
  },
  {
    title: "groupRandom",
    requires: "random",
    hint: "needsRandom",
    patterns: [
      ["${rstringalnum=8}", "tokAlnum"],
      ["${rstringalpha=8}", "tokAlpha"],
      ["${rstringdigit=6}", "tokDigit"],
      ["${ruuidv4}", "tokUuid"],
    ],
  },
  {
    title: "groupDate",
    patterns: [
      ["$YYYY", "tokYear4"],
      ["$YY", "tokYear2"],
      ["$Y", "tokYear1"],
      ["$MMMM", "tokMonthName"],
      ["$MMM", "tokMonthAbbr"],
      ["$MM", "tokMonth2"],
      ["$M", "tokMonth1"],
      ["$DDDD", "tokDayName"],
      ["$DDD", "tokDayAbbr"],
      ["$DD", "tokDay2"],
      ["$D", "tokDay1"],
      ["$hh", "tokHour2"],
      ["$h", "tokHour1"],
      ["$mm", "tokMinute2"],
      ["$m", "tokMinute1"],
      ["$ss", "tokSecond2"],
      ["$s", "tokSecond1"],
      ["$fff", "tokMilli3"],
      ["$ff", "tokMilli2"],
      ["$f", "tokMilli1"],
    ],
  },
  {
    title: "groupRegex",
    requires: "useRegex",
    hint: "needsRegex",
    patterns: [
      ["$1", "tokGroup"],
      ["$<name>", "tokNamed"],
      ["$&", "tokWhole"],
      ["$$", "tokDollar"],
    ],
  },
];

const REGEX_GROUPS: PatternGroup[] = [
  {
    title: "regexHelp",
    requires: "useRegex",
    hint: "needsRegex",
    patterns: [
      ["^", "rxStart"],
      ["$", "rxEnd"],
      [".*", "rxAll"],
      ["^foo", "rxStartsWith"],
      ["bar$", "rxEndsWith"],
      ["\\d+", "rxDigits"],
      ["\\s+", "rxSpaces"],
      ["(.*)", "rxGroup"],
      ["(\\d{2})-(\\d{2})-(\\d{4})", "rxSwapDate"],
      [".+?(?=bar)", "rxUpTo"],
      ["\\s*\\(\\d+\\)", "rxBrackets"],
    ],
  },
];

interface Props {
  options: RenameOptions;
  /** Inserts the pattern; `enable` turns on the option the pattern needs. */
  onInsert: (code: string, enable?: Partial<RenameOptions>) => void;
}

function PatternPopover({
  label,
  icon,
  groups,
  options,
  onInsert,
}: Props & { label: string; icon: ReactElement; groups: PatternGroup[] }) {
  const { t } = useI18n();
  const [open, setOpen] = useState(false);
  return (
    <Popover open={open} onOpenChange={(_, d) => setOpen(d.open)} positioning="below-end" withArrow>
      <PopoverTrigger disableButtonEnhancement>
        <Tooltip content={label} relationship="label">
          <Button appearance="transparent" size="small" icon={icon} />
        </Tooltip>
      </PopoverTrigger>
      <PopoverSurface className="pattern-surface">
        {groups.map((group) => {
          const missing = group.requires && !options[group.requires];
          return (
            <section key={group.title} className="pattern-group">
              <Text weight="semibold" block>
                {t(group.title)}
              </Text>
              {missing && group.hint && (
                <Text size={200} block className="hint">
                  {t(group.hint)}
                </Text>
              )}
              <div className="pattern-grid">
                {group.patterns.map(([code, description]) => (
                  <Fragment key={code}>
                    <Button
                      size="small"
                      appearance="subtle"
                      className="pattern-code"
                      onClick={() => {
                        onInsert(code, missing && group.requires ? { [group.requires]: true } : undefined);
                        setOpen(false);
                      }}
                    >
                      <code>{code}</code>
                    </Button>
                    <Text size={200}>{t(description)}</Text>
                  </Fragment>
                ))}
              </div>
            </section>
          );
        })}
      </PopoverSurface>
    </Popover>
  );
}

export function ReplacePatterns(props: Props) {
  const { t } = useI18n();
  return (
    <PatternPopover
      {...props}
      label={t("insertPattern")}
      icon={<BracesVariable20Regular />}
      groups={REPLACE_GROUPS}
    />
  );
}

export function RegexPatterns(props: Props) {
  const { t } = useI18n();
  return (
    <PatternPopover
      {...props}
      label={t("regexHelp")}
      icon={<QuestionCircle20Regular />}
      groups={REGEX_GROUPS}
    />
  );
}
