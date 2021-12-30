# Archive extension

A simple extension for quickly mass-archiving and organizing images from threads and archives.

## Installation

1. Download the extension from [the releases page](https://github.com/dagwaging/archive/releases/latest)
2. Extract `archive.zip` anywhere you like
3. Run `archive.exe` to install the extension's [native messaging host](https://developer.chrome.com/docs/apps/nativeMessaging/)
4. Open the [Chrome Extension page](chrome://extensions/) and click `Load unpacked`
5. Select the folder you extracted the extension to

## Configuration

1. Open the extension's [Chrome Extension page](chrome://extensions/?id=fdnmnpnjacfjphfmhlfgjpmkimbekmnd) and click on [`Extension options`](chrome://extensions/?options=fdnmnpnjacfjphfmhlfgjpmkimbekmnd)
2. Click `Choose folder` and select an `archive folder` to save images to

## Usage

1. Navigate to a thread and find a post with an image you want to archive, or hit `tab` to focus the first post
2. Type the name of the `subfolder` you want to save the image to into the textbox above the image. If a `subfolder` matches what you've typed so far, it will automatically be suggested. You can press `tab` to autocomplete the suggested `subfolder`.
3. Press enter to save the image. Images will be saved with their original filename, to the `archive folder` you specified in the extension options, within the `subfolder` you entered. The extension will then jump to the next post with an image.
4. Press `tab` to skip any images you don't want to archive

## Uninstallation

1. Open the extension's [Chrome Extension page](chrome://extensions/?id=fdnmnpnjacfjphfmhlfgjpmkimbekmnd)
2. Click `Remove extension` at the bottom of the page
3. Open the folder you extracted the extension to and run `archive.exe` to uninstall the extension's [native messaging host](https://developer.chrome.com/docs/apps/nativeMessaging/)
4. Delete the folder & zip file if desired
