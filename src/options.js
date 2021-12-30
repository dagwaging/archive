document.getElementById('directory').addEventListener('click', (event) => {
  chrome.runtime.sendNativeMessage(
    'com.dagwaging.archive',
    {
      "Pick": null
    },
    (response) => {
      let error = chrome.runtime.lastError
      if (error) {
        console.log({ error: error })
      }
      else {
        chrome.storage.local.set({ directory: response.msg }, () => {
          let error = chrome.runtime.lastError
          if (error) {
            console.log(error)
          }
        })
      }
    }
  )
})

document.getElementById('original_filename').addEventListener('change', (event) => {
  chrome.storage.local.set({ original_filename: event.target.checked }, () => {
    let error = chrome.runtime.lastError
    if (error) {
      console.log(error)
    }
  })
})

chrome.storage.onChanged.addListener((changes, areaName) => {
  if (areaName == 'local') {
    if (changes.directory !== undefined) {
      document.getElementById('label').innerText = changes.directory.newValue
    }

    if (changes.original_filename !== undefined) {
      document.getElementById('original_filename').checked = changes.original_filename.newValue
    }
  }
})

chrome.storage.local.get(['directory', 'original_filename'], (items) => {
  if (items.directory !== undefined) {
    document.getElementById('label').innerText = items.directory
  }

  document.getElementById('original_filename').checked = items.original_filename === undefined || items.original_filename
})
