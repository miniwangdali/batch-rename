; Remove the File Explorer context menu entry (if the user enabled it) when uninstalling.
!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCU "Software\Classes\*\shell\BatchRename"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\BatchRename"
!macroend
