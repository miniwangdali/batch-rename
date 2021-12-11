import { useState, useCallback, ChangeEventHandler } from "react";

export const useControlledSwitch = (
  initialValue: boolean
): [boolean, ChangeEventHandler<HTMLInputElement>] => {
  const [checked, setChecked] = useState(initialValue);
  const onChange: ChangeEventHandler<HTMLInputElement> = useCallback((evt) => {
    setChecked(evt.target.checked);
  }, []);
  return [checked, onChange];
};
