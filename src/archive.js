let suggestions = []

// pure-ish (not referentially transparent); call to get the suggestion for a partial name
let getSuggestion = (name) => {
  if (name.length > 0) {
    return suggestions.find(suggestion =>
      suggestion.startsWith(name)
    ) || ''
  }
  else {
    return ''
  }
}

// idempotent; called when new suggestions are added
let suggestionsAdded = (newSuggestions) => {
  newSuggestions.forEach((suggestion) => {
    if (!suggestions.includes(suggestion)) {
      suggestions.push(suggestion)
    }
  })
}


// idempotent; call to change the name associated with a file hash
let setName = (hash, name, original_filename, filename, url) => {
  if (name.length > 0) {
    chrome.storage.local.get('original_filename', (items) => {
      getPort().postMessage(
        {
          type: 'set',
          hash: hash,
          name: name,
          filename: (items.original_filename === undefined || items.original_filename) ? original_filename : filename,
          url: url
        }
      )
    })
  }
}

// idempotent; called when the name associated with some set of file hashes has changed
let nameChanged = (hashes) => {
  Object.entries(hashes).forEach(([hash, name]) => {
    let input = posts[hash]

    if (input) {
      input.value = name != null ? name : ''
      input.disabled = name != null
      input.placeholder = ''
    }
  })
}


let postsSelector = '.file'
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
  postsSelector = '.thread_image_box'
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

// pure; returns an input element with styling and events
let inputElement = (index, hash, original_filename, filename, url) => {
  let container = document.createElement('div')
  container.classList.add('archive-container')
  container.style.textAlign = 'left'

  let input = document.createElement('input')
  input.tabIndex = index
  input.disabled = true
  input.placeholder = 'Loading...'

  let suggestion = document.createElement('div')
  suggestion.style.textAlign = 'left'
  suggestion.style.position = 'absolute'
  suggestion.style.color = 'rgba(0, 0, 0, 0.5)'
  suggestion.style.fontSize = suggestionFontSize
  suggestion.style.lineHeight = suggestionLineHeight
  suggestion.style.padding = suggestionPadding

  input.addEventListener('keyup', (event) => {
    // technically also depends on the value of suggestions
    // so if they change, this should too, but meh
    suggestion.textContent = getSuggestion(event.target.value)
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

      setName(hash, event.target.value, original_filename, filename, url)

      let next = [...document.querySelectorAll('.thread ' + postsSelector + ' input')].find(
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

  return [container, input]
}

let posts = {}

// called when one or more new posts with images are loaded, idempotent
let postsChanged = async (nodes) => {
  if (nodes.length == 0) {
    return
  }

  let hashes = [...nodes].map((node) => {
    // assumptions: nodes is in order from top of page to bottom,
    // nodes are never added anywhere but to the bottom of the page
    let index = document.querySelectorAll('.thread ' + postsSelector + ' input').length + 1

    let link = linkSelector(node)
    let original_filename = link?.title || link?.text
    let filename = link?.pathname.split('/').at(-1)

    let hash = node.querySelector(hashSelector)?.getAttribute('data-md5')
    let url = node.querySelector(imageLinkSelector)?.href

    // this can happen if an image was removed from an archive page
    // TODO: pick a better postsSelector to avoid this situation?
    if (!original_filename || !filename || !hash) {
      return
    }

    let [container, input] = inputElement(index, hash, original_filename, filename, url)

    // not strictly idempotent but hopefully nobody else will ever mess with our container
    if (!node.querySelector('.archive-container')) {
      node.prepend(container)
    }

    // assumption: a post with a given hash will only appear once in the page
    // should always hold since 4chan does not allow duplicate files
    posts[hash] = input

    return hash
  }).filter(hash => hash)

  getPort().postMessage({
    type: 'get',
    hashes: hashes
  })
}


let port = null

// idempotent; returns a port to communicate with the extension background page
// connects the port if necessary and automatically reconnects if disconnected
let getPort = () => {
  if (!port) {
    let messageListener = (message, port) => {
      if (message.error) {
        if (message.error == 'No directory set') {
          Object.values(posts).forEach(input => {
            input.disabled = true
            input.placeholder = 'No directory set'
          })
        }
        else {
          console.error(message.error)
        }
      }
      else {
        switch (message.type) {
          case 'get':
            nameChanged(message.msg)
            break
          case 'suggestions':
            suggestionsAdded(message.msg)
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
        console.error(error)
      }
      else {
        getPort()
        console.log('Reconnected')
      }
    }

    port = chrome.runtime.connect()

    port.onMessage.addListener(messageListener)
    port.onDisconnect.addListener(disconnectListener)
  }

  return port
}

chrome.storage.onChanged.addListener((changes, areaName) => {
  if (areaName == 'local' && changes.directory) {
    Object.values(posts).forEach(input => {
      input.disabled = changes.directory.newValue != null
      input.placeholder = changes.directory.newValue != null ? '' : 'No directory set'
    })
    postsChanged(document.querySelectorAll('.thread ' + postsSelector))
  }
})

postsChanged(document.querySelectorAll('.thread ' + postsSelector))

new MutationObserver((mutations, observer) => {
  postsChanged(
    mutations.filter(mutation =>
      mutation.type === 'childList'
    ).flatMap(mutation =>
      // assumption: nodes are never removed
      [...mutation.addedNodes]
    ).map(node =>
      node.querySelector(postsSelector)
    ).filter(node => node)
  )
}).observe(document.querySelector('.thread'), { childList: true })
