let filesSelector = '.file'
let autoExpand = true
let linkSelector = (node) => {
  return node.querySelector('.fileText a')
}
let hashSelector = '.fileThumb img'
let imageLinkSelector = '.fileThumb'
let suggestionFontSize = '10pt'
let suggestionLineHeight = 'normal'
let suggestionPadding = '2px 3px'

if (window.location.origin !== 'https://boards.4chan.org') {
  filesSelector = '.thread_image_box'
  autoExpand = false
  linkSelector = (node) => {
    return node.parentElement.querySelector('a.post_file_filename')
  }
  hashSelector = 'img.post_image'
  imageLinkSelector = '.thread_image_link'
  suggestionFontSize = '13px'
  suggestionLineHeight = '18px'
  suggestionPadding = '3px 4px'
}

let sendMessageAsync = (message) => {
  return new Promise((resolve, reject) => {
    chrome.runtime.sendMessage(message, message => resolve(message))
  })
}

let suggestions = []

let files = {}

let addInputs = async (nodes) => {
  if (nodes.length == 0) {
    return
  }

  let hashes = [...nodes].map((node) => {
    let container = document.createElement('div')
    container.style.textAlign = 'left'

    let input = document.createElement('input')
    let index = document.querySelectorAll('.thread ' + filesSelector + ' input').length + 1
    input.tabIndex = index

    let suggestion = document.createElement('div')
    suggestion.style.textAlign = 'left'
    suggestion.style.position = 'absolute'
    suggestion.style.color = 'rgba(0, 0, 0, 0.5)'
    suggestion.style.fontSize = suggestionFontSize
    suggestion.style.lineHeight = suggestionLineHeight
    suggestion.style.padding = suggestionPadding

    let link = linkSelector(node)
    let hash = node.querySelector(hashSelector)?.getAttribute('data-md5')

    if (!link || !hash) {
      return
    }

    let file = {
      input: input,
      filename: link.title || link.text,
      url: node.querySelector(imageLinkSelector).href
    }

    files[hash] = file

    input.addEventListener('keyup', (event) => {
      if (event.target.value.length > 0) {
        suggestion.textContent = suggestions.find(name =>
          name.startsWith(event.target.value)
        ) || ''
      }
      else {
        suggestion.textContent = ''
      }
    })

    input.addEventListener('keydown', (event) => {
      if (event.which === 9 && suggestion.textContent.length > 0 && suggestion.textContent != event.target.value) {
        event.preventDefault()
        event.target.value = suggestion.textContent
      }
    })

    input.addEventListener('keypress', (event) => {
      if (event.which === 13) {
        event.preventDefault()

        if (event.target.value.length > 0) {
          // todo: detect duplicate files in multiple folders and remove others?
          getPort().postMessage(
            {
              type: 'set',
              hash: hash,
              name: event.target.value,
              filename: file.filename,
              url: file.url
            }
          )
        }

        let next = [...document.querySelectorAll('.thread ' + filesSelector + ' input')].find(
          input => input.tabIndex > index && !input.disabled
        )
        if (next) {
          next.focus()
        }
      }
    })

    input.addEventListener('focus', (event) => {
      let thumb = event.target.parentElement.parentElement.querySelector('.fileThumb img')
      if (thumb && thumb.style.display != 'none' && autoExpand) {
        thumb.click()
      }

      event.target.scrollIntoView(true)
    })

    input.addEventListener('blur', (event) => {
      let img = event.target.parentElement.parentElement.querySelector('.fileThumb img.expanded-thumb')
      if (img && autoExpand) {
        img.click()
      }
      suggestion.textContent = ''
    })

    container.append(suggestion)
    container.append(input)
    node.prepend(container)

    return hash
  }).filter(node => node)

  getPort().postMessage({
    type: 'get',
    hashes: hashes
  })
}

let port = null

let getPort = () => {
  if (port) {
    return port
  }
  else {
    let messageListener = (message, port) => {
      if (message.error) {
        console.log(message.error)
      }
      else {
        switch (message.type) {
          case 'get':
            Object.entries(message.msg).forEach(([hash, name]) => {
              if (!suggestions.includes(name)) {
                suggestions.push(name)
              }

              if (files[hash]) {
                files[hash].input.value = name
                files[hash].input.disabled = true
              }
            })
            break
          case 'suggestions':
            suggestions = message.msg
            break
        }
      }

      console.log('Message received', message)
    }

    let disconnectListener = (oldPort) => {
      let error = chrome.runtime.lastError

      oldPort.onMessage.removeListener(messageListener)
      oldPort.onDisconnect.removeListener(disconnectListener)
      port = null
      console.log('Disconnected')

      if (error) {
        console.log(error)
      }
      else {
        getPort()
        console.log('Reconnected')
      }
    }

    port = chrome.runtime.connect()

    port.onMessage.addListener(messageListener)
    port.onDisconnect.addListener(disconnectListener)

    return port
  }
}

addInputs(document.querySelectorAll('.thread ' + filesSelector))

new MutationObserver((mutations, observer) => {
  addInputs(
    mutations.filter(mutation =>
      mutation.type === 'childList'
    ).flatMap(mutation =>
      [...mutation.addedNodes]
    ).map(node =>
      node.querySelector(filesSelector)
    ).filter(file => file)
  )
}).observe(document.querySelector('.thread'), { childList: true })
