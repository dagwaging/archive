chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  switch (message.type) {
    case 'search':
      chrome.downloads.search(
        {},
        (results) => {
          sendResponse(
            results.filter(
              download =>
                download.byExtensionId == chrome.runtime.id &&
                download.exists &&
                download.state == 'complete'
            )
          )
        }
      )
      break
    case 'download':
      let filename = 'archive/' + message.name + '/' + message.filename // todo: make subdirectory name choosable
      chrome.downloads.download(
        {
          url: message.url,
          filename: filename,
          conflictAction: 'uniquify'
        },
        (downloadId) => {
          if (downloadId === undefined) {
            sendResponse(chrome.runtime.lastError + ': ' + filename)
          }
          else {
            sendResponse(null)
          }
        }
      )
      break
  }
  return true
})
