let connections = 0
let connection = null

chrome.storage.onChanged.addListener((changes, areaName) => {
  if (areaName == 'local' && changes.directory) {
    console.log(changes.directory.newValue)

    if (changes.directory.newValue) {
      if (!connection) {
        connection = chrome.runtime.connectNative('com.dagwaging.archive')
      }

      // our cache is probably invalid, we should eagerly rehash everything
      connection.postMessage({ "Get": { directory: changes.directory.newValue, hashes: [] } })
    }
  }
})

chrome.runtime.onConnect.addListener((port) => {
  if (!connection) {
    // TODO: if chrome is unable to communicate with the native host, this will silently fail
    connection = chrome.runtime.connectNative('com.dagwaging.archive')
  }

  let listener = (message, _) => {
    let error = chrome.runtime.lastError
    if (error) {
      port.postMessage({ error: error })
    }
    else {
      port.postMessage(message)
    }
  }

  connection.onMessage.addListener(listener)

  connections += 1
  console.log('Connected; ' + connections + ' connected total')

  port.onMessage.addListener((message, port) => {
    console.log('Message received', message)

    chrome.storage.local.get('directory', (items) => {
      if (!items.directory) {
        port.postMessage({ error: 'No directory set', detail: 'Go to the archive extension options page to set an archive directory' })
        return
      }

      try {
        switch (message.type) {
          case 'get':
            connection.postMessage({ "Get": { directory: items.directory, hashes: message.hashes } })
            break
          case 'set':
            connection.postMessage({
              "Set": {
                directory: items.directory,
                url: message.url,
                hash: message.hash,
                name: message.name,
                filename: message.filename
              }
            })
            break
        }
      }
      catch (error) {
        connection = null
        port.postMessage({ error: 'Extension not configured', detail: 'Run archive.exe to configure the extension' })
        return
      }
    })
  })

  port.onDisconnect.addListener((port) => {
    connection.onMessage.removeListener(listener)

    connections -= 1
    console.log('Disconnected; ' + connections + ' connected total')

    if (connections == 0) {
      connection.disconnect()
      connection = null
    }
  })
})
