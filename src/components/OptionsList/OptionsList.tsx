import _ from "lodash";
import {
  FormControlLabel,
  FormGroup,
  Switch,
  TextField,
  Typography
} from "@mui/material";
import React, { useEffect, useState } from "react";
import { useForm } from "react-hook-form";
import { generateNewFilename } from "../../utils/filename";
import { useControlledSwitch } from "../../hooks/ControlledSwitchHook";
import { OptionsListMessages as messages } from "./messages";
import { FormattedMessage, useIntl } from "react-intl";
import { useRecoilState } from "recoil";
import { filesState } from "../../atoms/files";
import "./OptionsList.scss";

const sample = "filename.ext";

type FormData = {
  newFilename: string;
};

export const OptionsList = () => {
  const intl = useIntl();
  const [files, setFiles] = useRecoilState(filesState);
  const {
    register,
    watch,
    formState: { errors }
  } = useForm<FormData>({ mode: "onChange" });
  const [newFilenamePattern, setNewFilenamePattern] = useState("");
  const [sampleFilename, setSampleFilename] = useState(sample);
  const [fillUp, onFillUpChange] = useControlledSwitch(false);
  const [overwriteExtension, onOverwriteExtensionChange] =
    useControlledSwitch(false);
  const { newFilename } = watch();

  useEffect(() => {
    if (typeof newFilename === "string") {
      if (newFilename.length === 0) {
        setSampleFilename(sample);
      }
      setNewFilenamePattern(newFilename);
    }
  }, [newFilename]);

  useEffect(() => {
    const updateSampleFilename = async () => {
      setSampleFilename(
        await generateNewFilename(newFilenamePattern, sample, 1, {
          fillUp,
          overwriteExtension: overwriteExtension,
          totalLength: files.length
        })
      );
    };
    updateSampleFilename();
    if (_.isEmpty(errors)) {
      const onFilePatternChange = async () => {
        const newFiles = [];
        for (let i = 0; i < files.length; i++) {
          const file = files[i];
          newFiles.push({
            ...file,
            newFileName: await generateNewFilename(
              newFilenamePattern,
              file.filename,
              i,
              { fillUp, overwriteExtension, totalLength: files.length }
            )
          });
        }
        setFiles(newFiles);
      };
      onFilePatternChange();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [newFilenamePattern, fillUp, overwriteExtension, files.length, errors]);

  return (
    <div className="OptionsList">
      <Typography
        className="OptionsList__sample-filename"
        variant="body1"
        paragraph
        gutterBottom
      >
        <FormattedMessage
          {...messages.sampleFilename}
          values={{ sample: sampleFilename }}
        />
      </Typography>
      <TextField
        id="OptionsList__new-filename"
        label={intl.formatMessage(messages.newFilename)}
        fullWidth
        autoComplete="off"
        size="small"
        error={Boolean(errors.newFilename)}
        helperText={
          Boolean(errors.newFilename)
            ? errors.newFilename?.message
            : intl.formatMessage(messages.newFilenameHelperText)
        }
        {...register("newFilename", {
          maxLength: 255,
          pattern: {
            value: /^[^\\/:?"<>|]+$/,
            message: intl.formatMessage(messages.patternErrorMessage)
          },
          validate: {
            onlyDots: (v) => {
              if (v.length === 0) return true;
              if (v.replaceAll(".", "").length === 0)
                return intl.formatMessage(messages.onlyDotErrorMessage);
              return true;
            }
          }
        })}
      />
      <FormGroup row>
        <FormControlLabel
          control={<Switch checked={fillUp} onChange={onFillUpChange} />}
          label={intl.formatMessage(messages.fillUpDigit)}
        />
        <FormControlLabel
          control={
            <Switch
              color="warning"
              checked={overwriteExtension}
              onChange={onOverwriteExtensionChange}
            />
          }
          label={intl.formatMessage(messages.overwriteExtension)}
        />
      </FormGroup>
    </div>
  );
};
