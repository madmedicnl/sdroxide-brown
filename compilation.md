# Guide de compilation locale (Windows, cette machine)

Vérifié le 2026-11-07 : check, build release, client web, tests i18n, `check --workspace --all-targets`,
check wasm, `cargo test --workspace`. Cible : `x86_64-pc-windows-gnu` ; les scripts de build hôte
utilisent la chaîne MSVC (`rust-toolchain.toml`, 1.98.1).

## 1. La règle : toujours passer par `sdr-cargo.sh`

```bash
bash /d/sdrfull/tools/sdr-cargo.sh check -p sdroxide-ui
bash /d/sdrfull/tools/sdr-cargo.sh build --release
bash /d/sdrfull/tools/sdr-cargo.sh test -p sdroxide-ism
```

Le script (hors dépôt, dans `D:\sdrfull\tools`) :

- règle l'environnement : `OPUS_URL`, `TEMP/TMP/TMPDIR=D:\tmp`, PATH avec `/c/msys64/mingw64/bin`,
  `CARGO_INCREMENTAL=0` ;
- ajoute `-j 16` et `--target x86_64-pc-windows-gnu` (sauf si `--target`/`-j` sont déjà donnés) ;
- avertit s'il reste moins de 20 Go sur `D:` ;
- gère seul le piège Opus (section 4) ;
- journal : `D:\tmp\sdr-cargo.log`. Le code retour est celui de `cargo`.

Pour le check wasm, passer la cible en argument (le script n'ajoute plus la cible gnu) :

```bash
bash /d/sdrfull/tools/sdr-cargo.sh check -p sdroxide-ui --target wasm32-unknown-unknown
```

## 2. Ordre complet de validation

| # | Commande (via `sdr-cargo.sh`) | Durée |
|---|---|---|
| 1 | `check -p sdroxide-ui` | ~2 min |
| 2 | `build --release` | ~2 min (+~5 min si nouvel `OUT_DIR` Opus) |
| 3 | client web, voir section 3 | ~10 min à froid |
| 4 | `test -p sdroxide-ui --lib i18n` | ~1 min (7 tests) |
| 5 | `check --workspace --all-targets` | ~6 min, silencieux = OK |
| 6 | `check -p sdroxide-ui --target wasm32-unknown-unknown` | ~2 min |

Prévenir avant tout ce qui dépasse 3 minutes. Lancer les longues en arrière-plan.
Tester uniquement les crates touchées : un `cargo test --workspace` complet dure ~1 h et remplit
~130 Go de `target`.

Binaire : `target/x86_64-pc-windows-gnu/release/sdroxide.exe`. Seule dépendance d'exécution non Windows :
`SoapySDR.dll` (dossier `bin` de PothosSDR, dans le PATH). Pas d'ALSA ni d'udev sous Windows.

## 3. Client web

`trunk` est installé (`~/.cargo/bin`), la cible `wasm32-unknown-unknown` aussi. Même environnement que
le script, avec `CARGO_BUILD_JOBS=16`. En arrière-plan (trunk télécharge `wasm-opt`, réseau requis) :

```bash
export OPUS_URL='D:/sdrfull/tools/opus/opus-pinned.zip' TEMP='D:\tmp' TMP='D:\tmp' TMPDIR='D:\tmp'
export PATH="/c/msys64/mingw64/bin:$PATH" CARGO_BUILD_JOBS=16 CARGO_INCREMENTAL=0
cd crates/sdroxide-web && trunk build --release     # sortie : crates/sdroxide-web/dist (wasm ~51 Mo)
```

Servir depuis le disque : `sdroxide.exe --server --web-root crates/sdroxide-web/dist`.
Pour l'embarquer dans l'exe : construire le web d'abord, puis `build --release --features embed-web`.
Utiliser le profil release de trunk : `cargo check --release --target wasm32-…` plante `rustc`
(`STATUS_HEAP_CORRUPTION`, problème de la machine).

## 4. Le piège Opus (`sdroxide-rade`)

Chaque nouvel `OUT_DIR` de `sdroxide-rade` (debug ou release, un par jeu de features, aussi dans un
worktree) relance Opus de zéro. Le Makefile généré lance `.\autogen.sh` via `cmd.exe` : Windows l'ouvre
avec son association (une fenêtre d'éditeur apparaît avec le script) et `make` attend indéfiniment,
à ~4 % CPU, sans `rustc`. **Fenêtre d'éditeur sur `autogen.sh` = c'est ça.**

Le script le détecte (journal figé 60 s sans le stamp `build_opus-configure`), arrête son `cargo`,
fait Opus à la main puis relance. Fermer la fenêtre d'éditeur si elle reste. À la main, si besoin :

```bash
# D = .../sdroxide-rade-<hash>/out/build/build_opus-prefix/src/build_opus  (le plus récent, sans stamp configure)
cp -f /d/sdrfull/tools/opus/opus_data-4ed9445b96698bad25d852e912b41495ddfa30c8dbc8a55f9cde5826ed793453.tar.gz "$D/"
MSYSTEM=MINGW64 TEMP='D:\tmp' TMP='D:\tmp' /c/msys64/usr/bin/bash.exe -lc \
  "cd <D> && ./autogen.sh && ./configure --with-pic --enable-osce --enable-dred --disable-shared \
   --disable-doc --disable-extra-programs && touch ../build_opus-stamp/build_opus-configure \
   && make -j16 && touch ../build_opus-stamp/build_opus-build"
```

Puis relancer `cargo` : il termine `rade` seul. Durée ~5 min, en arrière-plan.

- **Ne jamais télécharger** : `autogen.sh` tente `wget media.xiph.org` (IPv6, timeout). Le modèle est
  en local : `D:\sdrfull\tools\opus\opus_data-4ed9445b….tar.gz` (183 Mo). `opus-pinned.zip` est dans le
  même dossier (commit `940d4e5af64351ca8ba8390df3f555484c567fbb`). Si `vendor/rade_c/cmake/BuildOpus.cmake`
  réclame un autre commit, le zip est périmé.
- **Ne jamais effacer** les dossiers `sdroxide-rade-<hash>/out` (5 min à refaire).
- Ne jamais mettre `/c/msys64/usr/bin` dans le PATH de `cargo` (son `link.exe` masque celui de MSVC,
  erreur `LNK1104`). `autogen`/`make` d'Opus passent uniquement par le shell login MSYS2 ci-dessus.
- `mingw32-make` erreur 127 sur `celt/*.lo` = `sh` introuvable : Opus doit être compilé depuis le
  shell login MSYS2, pas avec le PATH du script.
- `CMake Error: generator MinGW Makefiles does not match … MSYS Makefiles` = `OUT_DIR` créé avec un
  autre PATH : utiliser le PATH du script, ou un `OUT_DIR` neuf.

## 5. Autres pièges connus

- **Cache wasm périmé** : erreur `E0004 non-exhaustive patterns: RadioEvent::QsoLog(_)` (variante
  inexistante dans les sources). Remède : `cargo clean -p sdroxide-types -p sdroxide-ui --target
  wasm32-unknown-unknown` (~560 Mo), puis relancer le check.
- **Fins de ligne** : les blobs sont en CRLF (`core.autocrlf=true`). `sed -i` sous Git Bash passe les
  fichiers en LF et le diff montre tout le fichier. Après un `sed -i` : restaurer avec
  `sed -i 's/\r$//; s/$/\r/' <fichier>` pour les fichiers listés par
  `git ls-files --eol | awk '$1=="i/crlf" && $2=="w/lf"'`, puis vérifier `git diff --numstat`.
- **Un seul filtre par `cargo test`** : `cargo test -p a -p b nom1 nom2` est refusé. Boucle `for`.
- **`python3 tools/i18n_add.py`** : le python MSYS n'a pas `fcntl` et lit stdin dans la page de code
  console. Passer par un petit wrapper hors dépôt qui neutralise `fcntl` (`flock` = no-op) avec
  `PYTHONUTF8=1` et le JSON dans un fichier UTF-8. Après un échec : `git restore crates/sdroxide-ui/i18n`.
- **Worktree neuf** : les 6 sous-modules `vendor/` y sont vides (`failed to read
  vendor\mfsk-core\mfsk-core\Cargo.toml`). Faire `git submodule update --init --recursive`, ou copier
  les dossiers depuis le dépôt principal et supprimer le fichier `.git` copié dans chacun. Pour réutiliser
  les artefacts : `CARGO_TARGET_DIR=/d/sdrfull/sdroxide-brown/target`.
- **`link` de `sdroxide-ism`** : `r_433` doit venir après le shim sur la ligne d'édition de liens
  (GNU ld lit les bibliothèques statiques une fois, dans l'ordre) et `netapi32` est requis. Corrigé
  dans `crates/sdroxide-ism/build.rs` (non commité).

## 6. Tests : résultats de référence

`cargo test --workspace` du 2026-11-07 (windows-gnu, ~1 h, 311 cibles) : **7402 réussis, 14 échoués,
115 ignorés** (non comparé à un `main` propre). Les 14 :

- connus sur `main` : `sdroxide-proto` ×3 (`the_dyndns_answer_is_appended_last`,
  `the_tx_refused_message_is_appended_last`, `the_wire_layout_of_the_config_structs_is_pinned`) ;
- artefacts CRLF : `sdroxide-winlink` `lzhuf` ×3 (`testdata/*.txt` en CRLF, références `.lzh` en LF) ;
  `sdroxide-ui` `every_state_of_the_operating_pane_has_a_way_out_of_it` (`frame.rs` cherche une chaîne
  contenant `\n` dans sa propre source). Un `testdata/*.txt -text` dans `.gitattributes` règlerait les
  premiers (PR seulement) ;
- passent seuls, échouent sous charge : `sdroxide-config` `the_cache_path_is_named_for_the_season…`,
  `sdroxide-hpsdr` `duc_pacing`, `sdroxide-pluto` `iiod_loopback` : les relancer seuls avant de
  soupçonner le code ;
- échouent aussi seuls, probablement Windows/réseau : `sdroxide-radio`
  `scanner::a_voice_hunt_stops_on_a_voice`, `sdroxide-server` `roster_edits`, `sdroxide-smartsdr`
  `loopback::an_eviction_for_a_duplicate_id…`, `sdroxide-tci` `a_refused_port_says_which_program_is_missing`.

`cargo test --workspace` demande ~50 Go libres minimum et ~130 Go au total : `No space left on device`
(os error 112) sinon. Les binaires `deps/*.exe` et `examples/*.exe` se suppriment ensuite (régénérables).

## 7. Disque (`D:` est petit) : nettoyage de fin de session

- Avant un gros build/test : `df -h /d` (viser > 20 Go ; > 60 Go pour `cargo test --workspace`).
- À la fin de chaque session, pour la suivante :

```bash
rm -rf target/x86_64-pc-windows-gnu/debug/incremental target/debug/incremental
```

  puis vider les restes de `D:\tmp` (`make*.bat`, `cc*`, dossiers `sdroxide-*` créés par les tests).
  Après un `cargo test --workspace` : supprimer aussi `target/x86_64-pc-windows-gnu/debug/deps/*.exe`,
  `.../debug/examples/*.exe` et les `*.pdb`.
- **Ne jamais supprimer** : `sdroxide-rade-<hash>/out`, `target/x86_64-pc-windows-gnu/release/sdroxide.exe`
  (sauf remplacement par un nouveau build), `crates/sdroxide-web/dist`, `D:\sdrfull\tools\opus\*`.
- Si encore juste : `cargo clean -p <crate>` pour les crates touchées, ou `target/debug` (ancien,
  côté hôte, ~19 Go), avant d'effacer tout `target`. Les `target` des worktrees (`vor-wt`, …) sont à
  décider cas par cas.

## 8. Rappels de dépôt qui touchent la compilation

- Porte avant PR : `check --workspace --all-targets` silencieux, tests des crates touchées seulement ;
  formater uniquement les fichiers touchés, jamais de `cargo fmt` global.
- Tout texte affiché passe par `tr!("zone-mots")` dans `sdroxide-ui`, ajouté dans les 11 langues ;
  lancer `test -p sdroxide-ui --lib i18n` avant tout push.
- Toucher au waterfall/zoom : garder verts les tests `ctr_zoom_sim` (`widgets/spectrum_view.rs`).
- Modifier une partie de code atteignable en wasm : vérifier aussi le check wasm.
- Branches à partir de `github/main` (`git fetch github`), pas de `origin/main` (miroir local, en retard).
