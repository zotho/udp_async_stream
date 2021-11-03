iotop:
	sudo iotop -P -t -b -u ${USER} -o -qqq -d 0 | rg 'target/release/udp_stream sink 0.0.0.0:8000 0.0.0.0:8001'

drop_cache:
	sudo su -c 'sync; echo 3 > /proc/sys/vm/drop_caches'