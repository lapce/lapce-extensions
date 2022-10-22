#!/bin/bash
if type flatpak-spawn >/dev/null 2>&1 ;
then
	DOCKER="flatpak-spawn --host docker"
	DOCKER_COMPOSE="flatpak-spawn --host docker-compose"
else
	DOCKER="docker"
	DOCKER_COMPOSE="docker-compose"
fi

function stop() {
	pkill cargo-watch
	$DOCKER_COMPOSE stop
}
if [ -z "$($DOCKER_COMPOSE top)" ];
then
	echo "[INFO] Starting microservices"
	$DOCKER_COMPOSE up -d
else
	echo "[INFO] Restarting microservices because they're already up"
	$DOCKER_COMPOSE restart
fi
echo "[INFO] Waiting 2s for the database to start"
sleep 2s
echo "[INFO] Running migrations"
cargo prisma db push
echo "[INFO] Setting shutdown trap"
trap stop 1 3 9 2
echo "[INFO] Starting backend"
cargo watch -x "run"
