import {
  FormControl,
  InputLabel,
  MenuItem,
  Select,
  SelectChangeEvent
} from "@mui/material";
import React from "react";
import { FormattedMessage } from "react-intl";
import { useRecoilState } from "recoil";
import { languageState } from "../../atoms/language";
import { SettingsPageMessages as messages } from "./messages";
import "./SettingsPage.scss";

export const SettingsPage = () => {
  const [lang, setLang] = useRecoilState(languageState);

  const handleChange = (evt: SelectChangeEvent) => {
    setLang(evt.target.value);
  };

  return (
    <div className="SettingsPage">
      <div className="SettingsPage__header">
        <h2 className="SettingsPage__title">
          <FormattedMessage {...messages.settings} />
        </h2>
      </div>
      <div className="SettingsPage__settings">
        <FormControl fullWidth>
          <InputLabel id="SettingsPage__language-label">
            <FormattedMessage {...messages.language} />
          </InputLabel>
          <Select
            labelId="SettingsPage__language-label"
            id="SettingsPage__language-select"
            value={lang}
            size="small"
            label={<FormattedMessage {...messages.language} />}
            onChange={handleChange}
          >
            <MenuItem value="zh-CN">简体中文</MenuItem>
            <MenuItem value="en-US">English(United States)</MenuItem>
          </Select>
        </FormControl>
      </div>
    </div>
  );
};
