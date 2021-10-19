let connections = 0
let connection = null

chrome.storage.onChanged.addListener((changes, areaName) => {
  if (areaName == 'local' && changes.directory) {
    // our cache is probably invalid, we should eagerly rehash everything
    console.log(changes.directory.newValue)
  }
})

chrome.runtime.onConnect.addListener((port) => {
  if (!connection) {
    connection = chrome.runtime.connectNative('com.dagwaging.archive')
  }

  let listener = (message, _) => {
    let error = chrome.runtime.lastError
    if (error) {
      port.postMessage({ error: error })
    }
    else {
      message.type = 'get'
      port.postMessage(message)

      if (message.msg) {
        port.postMessage({
          type: 'suggestions',
          msg: [...new Set(Object.values(message.msg))]
        })
      }
    }
  }

  connection.onMessage.addListener(listener)

  connections += 1
  console.log('Connected; ' + connections + ' connected total')

  port.onMessage.addListener((message, port) => {
    console.log('Message received', message)

    chrome.storage.local.get('directory', (items) => {
      if (!items.directory) {
        port.postMessage({ error: 'No directory set' })
        return
      }

      switch (message.type) {
        case 'get':
          connection.postMessage({ "Get": { directory: items.directory } })
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
