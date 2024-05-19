## where is the zip files
* `Lib.zip` --> https://github.com/RustPython/RustPython/tree/main/Lib
* `urllib3.zip` --> https://github.com/urllib3/urllib3/tree/main/src/urllib3
* `requests.zip` --> https://github.com/psf/requests/tree/main/src/requests


## generate build artifacts
```bash
pyoxidizer generate-python-embedding-artifacts build
```

## tips!
if you update `build/stdlib`, remember to zip again and replace `stdlib.zip`