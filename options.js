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

chrome.storage.onChanged.addListener((changes, areaName) => {
  if (areaName == 'local' && changes.directory) {
    document.getElementById('label').innerText = changes.directory.newValue
  }
})

chrome.storage.local.get('directory', (items) => {
  if (items.directory) {
    document.getElementById('label').innerText = items.directory
  }
})
