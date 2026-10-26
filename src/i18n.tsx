import { createContext, useContext, useMemo, type ReactNode } from "react";
import type { DateNames } from "./api";
import type { Language } from "./settings";
import en, { enOne, type Messages } from "./locales/en";
import zhCN from "./locales/zh-CN";

const dictionaries = { en, "zh-CN": zhCN } satisfies Record<string, Messages>;
const singulars: Partial<Record<keyof typeof dictionaries, Partial<Messages>>> = { en: enOne };

export type Locale = keyof typeof dictionaries;
export type MessageKey = keyof Messages;
export type Translate = (key: MessageKey, vars?: Record<string, string | number>) => string;

export function resolveLocale(language: Language): Locale {
  if (language !== "system") return language;
  return navigator.language.toLowerCase().startsWith("zh") ? "zh-CN" : "en";
}

function makeTranslate(locale: Locale): Translate {
  const messages = dictionaries[locale];
  const one = singulars[locale];
  return (key, vars) =>
    ((vars?.count === 1 && one?.[key]) || messages[key]).replace(/\{(\w+)\}/g, (match, name: string) =>
      vars && name in vars ? String(vars[name]) : match,
    );
}

/** Localized month/day names for the `$MMMM`, `$MMM`, `$DDDD` and `$DDD` patterns. */
export function dateNames(locale: Locale): DateNames {
  const format = (options: Intl.DateTimeFormatOptions, dates: Date[]) => {
    const f = new Intl.DateTimeFormat(locale, options);
    return dates.map((d) => f.format(d));
  };
  const months = Array.from({ length: 12 }, (_, m) => new Date(2001, m, 1));
  // 7 January 2001 was a Sunday.
  const days = Array.from({ length: 7 }, (_, d) => new Date(2001, 0, 7 + d));
  return {
    months: format({ month: "long" }, months),
    monthsShort: format({ month: "short" }, months),
    days: format({ weekday: "long" }, days),
    daysShort: format({ weekday: "short" }, days),
  };
}

interface I18n {
  locale: Locale;
  t: Translate;
}

const I18nContext = createContext<I18n>({ locale: "en", t: makeTranslate("en") });

export function I18nProvider({ locale, children }: { locale: Locale; children: ReactNode }) {
  const value = useMemo(() => ({ locale, t: makeTranslate(locale) }), [locale]);
  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export const useI18n = () => useContext(I18nContext);
