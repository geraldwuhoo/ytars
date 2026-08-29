# v0.8.1

Features

* Show scan progress on the scan page while a scan is running

Misc

* Replace OpenSSL with rustls, dropping the vendored build
* Shrink the container image by trimming codecs and stripping ffmpeg
* Simplify scan state ownership by removing Arc

Bugfixes

* Fix yt-dlp losing curl_cffi and brotli support in the container image

Chores

* Switch from jemallocator to tikv-jemallocator
