import React from "react";
import { FormattedMessage } from "react-intl";
import { OptionsList } from "../../components/OptionsList/OptionsList";
import { OptionsPageMessages as messages } from "./messages";
import "./OptionsPage.scss";

export const OptionsPage = () => {
  return (
    <div className="OptionsPage">
      <div className="OptionsPage__header">
        <h2 className="OptionsPage__title">
          <FormattedMessage {...messages.options} />
        </h2>
      </div>
      <OptionsList />
    </div>
  );
};
