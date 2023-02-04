#!/bin/bash
help() {
    echo "Copies the www content of the source directory to the given output"
    echo "directory. Headers and footers for the .html files in the given"
    echo "input directory are concatenated."
    echo ""
    echo "Usage:"
    echo "  $0 [-i|o]"
    echo ""
    echo -e "-i\tinput directory"
    echo -e "-o\toutput directory"
    echo ""
    exit
}

# defaults
www_src="ronaldos_webserver/www"
out_dir="/tmp"

while getopts ":h:o:i" option; do
   case $option in
      h) # display Help
         help
         exit;;
      o)
          out_dir=$OPTARG;;
      i)
          www_src=$OPTARG;;
     \?) # incorrect option
         echo "Error: Invalid option"
         help
         exit;;
   esac
done

echo "copying contents from ${www_src} to ${out_dir}"
cp -r "$www_src"/. "$out_dir"

filter="header.html footer.html"
for html in "$www_src"/*.html; do
    filename=$(basename "$html")
    if [[ "${filter}" =~ "$filename" ]]; then
        continue
    fi
    out_file=$out_dir/$filename
    echo "creating ${out_file}"
    cat "$www_src"/header.html > "$out_file"
    cat "$html" >> "$out_file"
    cat "$www_src"/footer.html >> "$out_file"
done
