# v0.8.0

Misc

* Speed up scans by bulk-loading existing videos and batching inserts
* Generate thumbnails in parallel
* Move blocking filesystem, image, and subprocess work off the async runtime
* Update askama and actix-files

Bugfixes

* Fix simultaneous scan requests starting duplicate scans
* Fix scan flag never clearing when a scan fails

Chores

* Various dependency updates
