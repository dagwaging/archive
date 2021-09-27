let names = []

let addInput = (node) => {
  let container = document.createElement('div')
  let input = document.createElement('input')
  let index = document.querySelectorAll('.thread .file input').length + 1
  input.tabIndex = index

  let suggestion = document.createElement('div')
  suggestion.style.position = 'absolute'
  suggestion.style.padding = '2px 3px'
  suggestion.style.color = 'rgba(0, 0, 0, 0.5)'

  input.addEventListener('keyup', (event) => {
    if (event.target.value.length > 0) {
      let name = names.find(name => name.startsWith(event.target.value))
      if (name) {
        suggestion.textContent = name
      }
      else {
        suggestion.textContent = ''
      }
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
        let link = event.target.parentElement.parentElement.querySelector('.fileText a')
        chrome.runtime.sendMessage(
          {
            type: 'download',
            name: event.target.value,
            filename: link.title || link.text, // todo: use md5 hash as filename?
            url: event.target.parentElement.parentElement.querySelector('.fileThumb').href
          },
          (error) => {
            if (error == null) {
              if (!names.includes(event.target.value)) {
                names.push(event.target.value)
              }
              event.target.disabled = true
            }
            else {
              console.log(error) // todo: allow picking a different name if name is invalid, or pick automatically
            }
          }
        )
      }

      let next = [...document.querySelectorAll('.file input')].find(
        input => input.tabIndex > index && !input.disabled
      )
      if (next) {
        next.focus()
      }
    }
  })

  input.addEventListener('focus', (event) => {
    let thumb = event.target.parentElement.parentElement.querySelector('.fileThumb img')
    if (thumb.style.display != 'none') {
      thumb.click()
    }

    event.target.scrollIntoView(true)
  })

  input.addEventListener('blur', (event) => {
    let img = event.target.parentElement.parentElement.querySelector('.fileThumb img.expanded-thumb')
    if (img) {
      img.click()
    }
    suggestion.textContent = ''
  })

  container.append(suggestion)
  container.append(input)
  node.prepend(container)
}

document.querySelectorAll('.thread .file').forEach(addInput)

// file existence is only checked once, on page load
// only files downloaded using the extension can be detected
chrome.runtime.sendMessage(
  {
    type: 'search'
  },
  (results) => {
    results.forEach((result) => {
      let path = result.filename.split('\\')
      let name = path[path.length - 2]

      if (!names.includes(name)) {
        names.push(name)
      }

      // todo: check files by md5 hash rather than original url or filename?
      let file = document.querySelector('.fileText a[href="' + result.url.replace(/^https:/, '') + '"]')
      if (file) {
        let input = file.parentElement.parentElement.querySelector('input')
        input.value = name
        input.disabled = true
      }
    })
  }
)

new MutationObserver((mutations, observer) => {
  mutations.filter((mutation) => {
    return mutation.type === 'childList'
  }).forEach((mutation) => {
    mutation.addedNodes.forEach((node) => {
      let file = node.querySelector('.file')
      if (file) {
        addInput(file)
      }
    })
  })
}).observe(document.querySelector('.thread'), { childList: true })
