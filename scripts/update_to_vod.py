import argparse
import shutil
import re
import os
import subprocess

parser = argparse.ArgumentParser()
parser.add_argument(dest='folder', help="the folder which contains the various hls streams + playlist files")
args = parser.parse_args()

if not os.path.isdir(args.folder):
    print(f"{args.folder} is not a directory. exiting..")
    exit(1)

seq_re = re.compile(r"#EXT-X-MEDIA-SEQUENCE:\d+", re.IGNORECASE)
vod_re = re.compile(r"#EXT-X-INDEPENDENT-SEGMENTS", re.IGNORECASE)

def rewrite_playlist(playlist, files):
    output = []
    with open(playlist, 'w') as file:
        output = "#EXTM3U\n\
#EXT-X-VERSION:3\n\
#EXT-X-TARGETDURATION:6\n\
#EXT-X-MEDIA-SEQUENCE:0\n\
#EXT-X-PLAYLIST-TYPE:VOD\n"
        segment_count = 0
        segment= f"{str(count)}_{'{:02}'.format(segment_count)}.ts"
        while segment in files:
            duration= subprocess.check_output(f'ffprobe -i {segment} -show_entries format=duration -v quiet -of csv="p=0"', shell=True, encoding='utf-8')
            output += f"#EXTINF:{str(duration.strip())},\n"
            output += f"{ segment }\n"
            print(f"{ segment }")
            segment_count += 1
            segment= f"{str(count)}_{'{:02}'.format(segment_count)}.ts"
        output += "#EXT-X-ENDLIST\n"
        file.write(output)


files = set(os.listdir(args.folder))
previous_dir = os.getcwd()
os.chdir(args.folder)

count = 0
playlist= f"{count}.m3u8"
while playlist in files:
    shutil.copyfile(playlist, f"{playlist}_bak")
    rewrite_playlist(playlist, files)
    count +=1
    playlist= f"{count}.m3u8"

os.chdir(previous_dir)