#!/bin/bash
function stop() {
	pkill cargo-watch
	docker-compose stop
}
if [ -z "$(docker-compose top)" ];
then
	docker-compose up -d
else
	docker-compose restart
fi
sleep 1s
diesel migration run
trap stop 1 3 9 2
cargo watch -x "run --bin server" -w server
