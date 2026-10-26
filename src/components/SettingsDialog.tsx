import { useCallback, useEffect, useState } from "react";
import {
  Button,
  Dialog,
  DialogActions,
  DialogBody,
  DialogContent,
  DialogSurface,
  DialogTitle,
  DialogTrigger,
  Field,
  Select,
  Spinner,
  Switch,
  Text,
} from "@fluentui/react-components";
import { version } from "../../package.json";
import { api, CANCELLED, type IntegrationStatus } from "../api";
import { useI18n } from "../i18n";
import type { Language, Settings, Theme } from "../settings";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  settings: Settings;
  onChange: (settings: Settings) => void;
}

interface Message {
  intent: "error" | "warning";
  text: string;
}

export function SettingsDialog({ open, onOpenChange, settings, onChange }: Props) {
  const { t } = useI18n();
  const [integration, setIntegration] = useState<IntegrationStatus | null>(null);
  const [busy, setBusy] = useState(false);
  const [menuMessage, setMenuMessage] = useState<Message | null>(null);
  const [startMenuError, setStartMenuError] = useState<string | null>(null);

  const refresh = useCallback(
    () => api.integrationStatus().then(setIntegration, () => setIntegration(null)),
    [],
  );

  useEffect(() => {
    if (open) {
      setMenuMessage(null);
      setStartMenuError(null);
      refresh();
    }
  }, [open, refresh]);

  const set = <K extends keyof Settings>(key: K, value: Settings[K]) =>
    onChange({ ...settings, [key]: value });

  const setContextMenu = async (enabled: boolean) => {
    setBusy(true);
    setMenuMessage(null);
    try {
      await api.setContextMenu(enabled, t("shellMenuLabel"));
    } catch (e) {
      setMenuMessage(
        e === CANCELLED
          ? { intent: "warning", text: t("contextMenuCancelled") }
          : { intent: "error", text: String(e) },
      );
    } finally {
      await refresh();
      setBusy(false);
    }
  };

  const setStartMenu = async (enabled: boolean) => {
    try {
      await api.setStartMenu(enabled);
      setStartMenuError(null);
    } catch (e) {
      setStartMenuError(String(e));
    }
    await refresh();
  };

  return (
    <Dialog open={open} onOpenChange={(_, d) => onOpenChange(d.open)}>
      <DialogSurface>
        <DialogBody>
          <DialogTitle>{t("settingsTitle")}</DialogTitle>
          <DialogContent className="settings">
            <Field label={t("language")}>
              <Select
                value={settings.language}
                onChange={(_, d) => set("language", d.value as Language)}
              >
                <option value="system">{t("languageSystem")}</option>
                <option value="en">English</option>
                <option value="zh-CN">简体中文</option>
              </Select>
            </Field>
            <Field label={t("theme")}>
              <Select value={settings.theme} onChange={(_, d) => set("theme", d.value as Theme)}>
                <option value="system">{t("themeSystem")}</option>
                <option value="light">{t("themeLight")}</option>
                <option value="dark">{t("themeDark")}</option>
              </Select>
            </Field>
            <Switch
              checked={settings.restoreOptions}
              onChange={(_, d) => set("restoreOptions", d.checked)}
              label={t("restoreOptions")}
            />
            <Switch
              checked={settings.fillLastUsed}
              onChange={(_, d) => set("fillLastUsed", d.checked)}
              label={t("fillLastUsed")}
            />
            <Switch
              checked={settings.rememberRecent}
              onChange={(_, d) => set("rememberRecent", d.checked)}
              label={t("rememberRecent")}
            />
            {settings.rememberRecent && (
              <Field label={t("maxRecent")} orientation="horizontal" className="indented">
                <Select
                  value={String(settings.maxRecent)}
                  onChange={(_, d) => set("maxRecent", Number(d.value))}
                >
                  {[5, 10, 20, 50].map((n) => (
                    <option key={n} value={n}>
                      {n}
                    </option>
                  ))}
                </Select>
              </Field>
            )}
            <Switch
              checked={settings.closeAfterRename}
              onChange={(_, d) => set("closeAfterRename", d.checked)}
              label={t("closeAfterRename")}
            />
            {integration?.startMenu != null && (
              <Field
                validationState={startMenuError ? "error" : "none"}
                validationMessage={startMenuError ?? undefined}
              >
                <Switch
                  checked={integration.startMenu}
                  onChange={(_, d) => setStartMenu(d.checked)}
                  label={t("startMenu")}
                />
              </Field>
            )}
            {integration?.contextMenu != null && (
              <Field
                hint={integration.contextMenuNeedsAdmin ? t("contextMenuHint") : undefined}
                validationState={menuMessage?.intent ?? "none"}
                validationMessage={menuMessage?.text}
              >
                <div className="field-row">
                  <Switch
                    checked={integration.contextMenu}
                    disabled={busy}
                    onChange={(_, d) => setContextMenu(d.checked)}
                    label={t("contextMenu")}
                  />
                  {busy && <Spinner size="tiny" />}
                </div>
              </Field>
            )}
            <Text size={200} className="hint">
              {t("version", { version })}
            </Text>
          </DialogContent>
          <DialogActions>
            <DialogTrigger disableButtonEnhancement>
              <Button appearance="primary">{t("close")}</Button>
            </DialogTrigger>
          </DialogActions>
        </DialogBody>
      </DialogSurface>
    </Dialog>
  );
}
