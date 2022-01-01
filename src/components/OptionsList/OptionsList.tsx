import _ from "lodash";
import {
  Box,
  FormControlLabel,
  FormGroup,
  Switch,
  TextField,
  Typography
} from "@mui/material";
import React, { ChangeEventHandler, useEffect, useState } from "react";
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
  const [fillUpLength, setFillUpLength] = useState(files.length);
  const [overwriteExtension, onOverwriteExtensionChange] =
    useControlledSwitch(false);
  const { newFilename } = watch();

  useEffect(() => {
    setFillUpLength(Math.floor(files.length / 10 + 1));
  }, [files.length]);

  useEffect(() => {
    if (typeof newFilename === "string") {
      if (newFilename.length === 0) {
        setSampleFilename(sample);
      }
      setNewFilenamePattern(newFilename);
    }
  }, [newFilename]);

  useEffect(() => {
    console.log(fillUpLength);
    const updateSampleFilename = async () => {
      setSampleFilename(
        await generateNewFilename(newFilenamePattern, sample, 1, {
          fillUp,
          overwriteExtension: overwriteExtension,
          fillUpLength
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
              { fillUp, overwriteExtension, fillUpLength }
            )
          });
        }
        setFiles(newFiles);
      };
      onFilePatternChange();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [newFilenamePattern, fillUp, fillUpLength, overwriteExtension, errors]);

  const onFillUpLengthChange: ChangeEventHandler<HTMLInputElement> = (evt) => {
    const length = parseInt(evt.target.value);
    setFillUpLength(Number.isNaN(length) ? 0 : length);
  };

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
      <FormGroup>
        <Box sx={{ display: "flex" }}>
          <FormControlLabel
            control={<Switch checked={fillUp} onChange={onFillUpChange} />}
            label={intl.formatMessage(messages.fillUpDigit)}
          />
          <TextField
            id="OptionsList__fill-up-length-input"
            disabled={!fillUp}
            value={fillUpLength}
            onChange={onFillUpLengthChange}
            size="small"
            variant="outlined"
            type="number"
            sx={{ width: "50%" }}
            inputProps={{ inputMode: "numeric", pattern: "[0-9]*" }}
          />
        </Box>

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
