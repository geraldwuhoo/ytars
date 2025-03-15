#!/usr/bin/env python

import argparse

import yaml

import yt_dlp

parser = argparse.ArgumentParser()
parser.add_argument(
    "--path",
    default=".",
    help="Directory path to download videos to",
)
parser.add_argument(
    "--list",
    default="list.yaml",
    help="Path to list.yaml file",
)
parser.add_argument(
    "-q",
    "--quiet",
    default=False,
    action=argparse.BooleanOptionalAction,
    help="Quiet mode for yt-dlp",
)
parser.add_argument(
    "--progress",
    default=True,
    action=argparse.BooleanOptionalAction,
    help="Print progress for yt-dlp",
)
parser.add_argument(
    "--channel",
    default="",
    help="Download single channel",
)
args = parser.parse_args()

with open(args.list, "r", encoding="utf8") as stream:
    data = yaml.safe_load(stream)

for channel in data:
    name = channel["name"]
    url = channel["url"]
    livestream = channel.get("livestreams", False)
    quality = channel.get("quality")
    match quality:
        case "1440p":
            # Download in priority:
            # 1. webm compatible: 1440p > 1080p > 720p, AV1 > VP9, HFR preferred, OPUS
            # 2. mp4 compatible: 1080p and lower, H264, M4A
            # To avoid .mkv files (mainly h264 + opus), which cannot be played in the browser
            ytdlp_format = "(700/400/308/271/699/399/303/248/616/698/398/302/612/247)+bestaudio[acodec=opus]/bestvideo[vcodec^=avc1]+bestaudio[ext=m4a]/bestvideo+bestaudio/best"
        case "1080p":
            # Download in priority:
            # 1. webm compatible: 1080p > 720p, AV1 > VP9, HFR preferred, OPUS
            # 2. mp4 compatible: 1080p and lower, H264, M4A
            # To avoid .mkv files (mainly h264 + opus), which cannot be played in the browser
            ytdlp_format = "(699/399/303/248/616/698/398/302/612/247)+bestaudio[acodec=opus]/bestvideo[vcodec^=avc1]+bestaudio[ext=m4a]/bestvideo+bestaudio/best"
        case "720p":
            # Download in priority:
            # 1. webm compatible: 720p, AV1 > VP9, HFR preferred, OPUS
            # 2. mp4 compatible: 720p and lower, H264, M4A
            # To avoid .mkv files (mainly h264 + opus), which cannot be played in the browser
            ytdlp_format = "(698/398/302/612/247)+bestaudio[acodec=opus]/bestvideo[height<=?720][vcodec^=avc1]+bestaudio[ext=m4a]/bestvideo+bestaudio/best"
        case _:
            # Download in priority:
            # 1. webm compatible: 4320p > 2160p > 1440p > 1080p > 720p, AV1 > VP9, HFR preferred, OPUS
            # 2. mp4 compatible: 1080p and lower, H264, M4A
            # To avoid .mkv files (mainly h264 + opus), which cannot be played in the browser
            ytdlp_format = "(702/571/402/272/701/401/315/313/700/400/308/271/699/399/303/248/616/698/398/302/612/247)+bestaudio[acodec=opus]/bestvideo[vcodec^=avc1]+bestaudio[ext=m4a]/bestvideo+bestaudio/best"

    if args.channel != "" and args.channel != name:
        continue

    ydl_opts = {
        "outtmpl": {"default": f"{args.path}/{name}/%(title)s [%(id)s].%(ext)s"},
        "writedescription": True,
        "writeinfojson": True,
        "writethumbnail": True,
        "writesubtitles": True,
        "writeautomaticsub": True,
        "download_archive": f"{args.path}/archive",
        "progress": args.progress,
        "quiet": args.quiet,
        "ignoreerrors": True,
        "match_filter": yt_dlp.utils.match_filter_func(
            # Ignore all currently live or upcoming livestreams, as these are useless to download
            "live_status !=? is_live & live_status !=? is_upcoming{}".format(
                # By default, ignore all livestreams
                ""
                if livestream
                else " & live_status !=? was_live"
            )
        ),
        "format": ytdlp_format,
    }
    with yt_dlp.YoutubeDL(ydl_opts) as ydl:
        print(f"Downloading: {name} ({url})")
        try:
            ydl.download(url)
        except Exception:
            continue
