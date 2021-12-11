import { defineMessages } from "react-intl";

export const OptionsListMessages = defineMessages({
  fillUpDigit: {
    id: "components.OptionsList.OptionsList.fillUpDigit",
    defaultMessage: "补齐位数"
  },
  newFilename: {
    id: "components.OptionsList.OptionsList.newFilename",
    defaultMessage: "新文件名"
  },
  newFilenameHelperText: {
    id: "components.OptionsList.OptionsList.newFilenameHelperText",
    defaultMessage: "* - 原文件名, # - 文件序号"
  },
  onlyDotErrorMessage: {
    id: "components.OptionsList.OptionsList.onlyDotErrorMessage",
    defaultMessage: "文件名不能只包含“.”"
  },
  overwriteExtension: {
    id: "components.OptionsList.OptionsList.overwriteExtension",
    defaultMessage: "修改扩展名"
  },
  patternErrorMessage: {
    id: "components.OptionsList.OptionsList.patternErrorMessage",
    defaultMessage: '文件名不能包含以下字符:\\ / : ?" < > |'
  },
  sampleFilename: {
    id: "components.OptionsList.OptionsList.sampleFilename",
    defaultMessage: "新文件名示例：{sample}"
  }
});
