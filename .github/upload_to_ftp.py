import sys
from ftplib import FTP
import os

def directory_exists(dir):
    if not dir:
        return True
    filelist = []
    ftp.retrlines('LIST',filelist.append)
    for f in filelist:
        if f.split()[-1] == dir and f.upper().startswith('D'):
            return True
    return False

def remove_hidden_folders(files):
    for f in files:
        if '.' in os.path.dirname(f):
            files.remove(f)

changed_files = [] 
removed_files = []

if (len(sys.argv) > 4):
    changed_files = sys.argv[4].split(',')
    print("changed/added files:{}".format(changed_files))

if (len(sys.argv) > 5):
    removed_files = sys.argv[5].split(',')
    print("removed files:{}".format(removed_files))

remove_hidden_folders(changed_files)

ftp = FTP(host=sys.argv[1], user=sys.argv[2], passwd=sys.argv[3])
ftp.cwd('/opt/share/www/')

for fl in changed_files:
    dir = os.path.dirname(fl)
    if directory_exists(dir) is False:
        print("creating dir: " + dir)
        ftp.mkd(dir)
    print("uploading " + fl)
    ftp.storbinary('STOR '+fl, open(fl, 'rb'))

for f in removed_files:
    print("deleting: " + f)
    ftp.delete(f)
    dir = os.path.dirname(f)
    if (len(ftp.nlst(dir)) <= 2):
        print("removing dir: " + dir)
        ftp.rmd(dir)
ftp.quit()