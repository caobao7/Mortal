#/usr/bin/sh
if [ $# -ne 1 ];then
	echo "$0 file"
	exit
fi
file=$1
cur_dir=$(dirname $(realpath $0))
#cp ${file}  "${cur_dir}/${file}"
#gzip -d "${cur_dir}/${file}"
cp "${cur_dir}/index.example.html" "${cur_dir}/index.html"
echo "" > "${cur_dir}/index.html" 
cat "${cur_dir}/pa" >> "${cur_dir}/index.html" 
zcat -c "${file}" >> "${cur_dir}/index.html" 
cat "${cur_dir}/pb" >> "${cur_dir}/index.html" 
cp "${cur_dir}/index.html" "/mnt/c/Users/22683/Desktop/log-viewer/" 
