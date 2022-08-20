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

files = set(os.listdir(args.folder))
seq_re = re.compile(r"#EXT-X-MEDIA-SEQUENCE:\d+", re.IGNORECASE)
previous_dir = os.getcwd()
os.chdir(args.folder)

def run():
    count = 0
    playlist= f"{count}.m3u8"
    while playlist in files:
        shutil.copyfile(playlist, f"{playlist}_bak",)

        with open(playlist, 'w+') as file:
            text = file.read()
            text = seq_re.sub('#EXT-X-MEDIA-SEQUENCE:0', text,1)
            pos = text.find('#EXT-X-INDEPENDENT-SEGMENTS')
            output = text[:pos]
            output += "#EXT-X-PLAYLIST-TYPE:VOD\n"

            segment_count = 0
            segment= f"{str(count)}_{'{:02}'.format(segment_count)}.ts"
            while segment in files:
                segment_count += 1
                segment= f"{str(count)}_{'{:02}'.format(segment_count)}.ts"
                duration= subprocess.check_output(f'ffprobe -i {segment} -show_entries format=duration -v quiet -of csv="p=0"', shell=True, encoding='utf-8')
                output += f"#EXTINF:{str(duration.strip())},\n"
                output += f"{ segment },\n"
            print(output)
            playlist.write(output)

        count +=1
        playlist= f"{count}.m3u8"

try:
    run()
finally:
     os.chdir(previous_dir)