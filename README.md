This project is a application to batch rename files.

## Available Scripts

In the project directory, you can run:

### `yarn tauri:dev`

Runs the app in the development mode.\
It will open a browser window at [http://localhost:3000](http://localhost:3000). But that won't work due to lack of built-in window support.
Once it finishes compiling, it will also open a application window. This window should show the app correctly.
The page will reload if you make edits.\
You will also see any lint errors in the console.

### `yarn tauri:build`

Builds a release for the app.
The release is under `src-tauri\target\release`. The installer is under `src-tauri\target\release\bundle`.

### `yarn tauri:build:dev`

Builds a development release for debugging purpose.
The release is under `src-tauri\target\debug`. The installer is under `src-tauri\target\debug\bundle`.


### `yarn translations:extract-compile`

Extract and compile all the messages used in the app to `zh_CN.json` under `src\lang` folder.
If there are other languages available, they need manually updates according to the new `zh_CN.json`.

## Licence

MIT License
