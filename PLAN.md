# bitchat-launcher : décisions et journal

Lanceur commun de bitchat-geohash (la carte) et de bitchat-console (par-dessus), demandé le
28/10/2026 : port 8080 ou port libre entre 50000 et 60000, HTTPS, `BITCHAT_TOR_SOCKS` puis le
port par défaut du démon tor puis Arti, `--help` (options usuelles d'abord), manuel détaillé
avec les options de chaque outil.

## Décisions

Toutes *prises par l'agent, à revoir*, sauf celles de la demande.

| N° | Sujet | Décision |
|----|-------|----------|
| L1 | Lien entre les outils | Deux processus enfants, pas de bibliothèque liée : bitchat-geohash 0.1.0 et Arti exigent deux versions de SQLite incompatibles (D26 de la console). |
| L2 | Ports | Page : `--port`, sinon 8080, sinon un port libre de 50000–60000 parcouru depuis un point pseudo-aléatoire (demande). Carte : un autre port libre de la même plage, sur 127.0.0.1, en HTTP clair, joint par la console seule. Un `--port` explicite occupé est une erreur. |
| L3 | Tor | `$BITCHAT_TOR_SOCKS`, puis 127.0.0.1:9050, puis Arti (demande). « Marche » = accepte un nom d'utilisateur SOCKS (l'isolation par cellule de la console) et ouvre un flux vers check.torproject.org:443 en moins de 30 s. `--tor-socks` et `--arti` explicites : pas de repli. La carte n'utilise pas Tor (elle ne publie jamais). |
| L4 | HTTPS | Par défaut (demande) : certificat localhost/127.0.0.1/::1 fait une fois par mkcert dans `~/.bitchat-launcher/tls` (clé 0600, dossier 0700) ; avertissement si l'autorité mkcert n'est pas installée ; `--tls-cert`/`--tls-key` ou `--http` sinon. |
| L5 | Options des outils | `--console-args '…'` et `--map-args '…'`, découpés comme par un shell, ajoutés après celles du lanceur ; les options que le lanceur fixe (`--listen`, TLS, `--map`, Tor, `--open`, `--allow-remote`…) sont refusées. Les plus usuelles sont aussi des options du lanceur : `--teleport`, `--depth`, `--verbose`. |
| L6 | Binaires | `--console-bin`/`--map-bin`, sinon à côté du lanceur, sinon `$PATH` (`cargo install`). |
| L7 | Zone de la carte | Par défaut le geohash de la console sans sa dernière lettre (`u09tv` → `u09t`), `--map-geohash` sinon. |
| L8 | Arrêt | Chaque outil dans son groupe de processus ; Ctrl-C ou SIGTERM → SIGINT à la console puis à la carte, 20 s chacun, puis SIGKILL. Un outil qui s'arrête arrête l'autre ; code 1. |
| L9 | Navigateur | Ouvert par défaut quand les deux répondent (`--no-open`), sur `https://localhost:<port>/`. |
| L10 | Base de la carte | `~/.bitchat-launcher/bitchat-geohash.sqlite` (et non le dossier courant) ; `--map-args '--db …'` sinon. |
| L11 | Dépendances | `tokio` (processus, signaux, TCP) et `libc` (SIGINT à un enfant). Pas de bibliothèque d'arguments : même approche à la main que les deux outils. |

## Journal

| Date | Fait |
|------|------|
| 28/10/2026 | Crate créée (args, ports, tor, tls, tools), `--help`, manuel, 22 tests hors ligne. Essais réels : Arti en repli quand le démon était arrêté ; démon par 127.0.0.1:9050 ; `BITCHAT_TOR_SOCKS` mort → 9050 ; 8080 occupé → 54586 ; HTTPS en HTTP/2 avec certificat de confiance, carte et console servies, `/api/state` ; arrêt propre. A révélé un défaut de bitchat-console (un flux de la carte relayé retenait l'arrêt 20 s), corrigé dans la console. Limite connue : deux lanceurs lancés au même instant peuvent choisir le même port ; le second s'arrête alors avec un message. |
