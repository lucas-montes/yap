You can't sync a remote db to a local one, only the other way around.
We'll focus on doing the writes remotely then.

# Garder deux historiques:
## Remote first
Un historique avec les fichiers qui ont eté sauvegardé sur le storage.
Distribue et en ligne. (Remote first, ecrire directement sur la db remote).
En quelleque sorte c'est la branch "master" celle qui vas en prod. Un des veersions stageging

## Local First
Travailler en loca, et lors d'un push, pusher seulement la dérvière vérsion en local.
Un historique des experiences locales.
(Local first, ecrire en local et plus tard si on veut, faire un push et creer une version distribue) 
Demander si cette option veut
etrê activée. Garder une copie des fichiers pour les comparer entre eux.

## Three steps:
* Add
* Commit
* Push

## Add
Starts to keep track of a file.
It creates the first entry in the local database.
Ask if we want to keep a copy of the files locally to compare them on commit.

## Commit
- Creates a new entry in the database and compares the two files. 

### Comparaison
Ask if we want to do that automatically for pipelines and/or CLI.
If we have the previous version compare the results. Compare them: 
- By hash
- If it's a parquet, columnar or something like so, use polars.
- If it's json, create structs, iter over k,v and compare.
- If it's plain text something like git.

### Metadata
- If it's a pipeline, keep track of the whole graph?
- Timestamp
- Commit message
- If it's a git repo, get the commit hash? Or other info related to the git state?
- Have some info of the user:
    - Info from git if posible
    - For turso to handle users and differnts groups
    - For the remote storage. Probably to be able to have the credentials to push and pull files

## Push
- For production pipelines, push remote would be master and we would write directly to the remote db.
Except if manual intervention is needed. Saving after each pipeline, directly to the remote db would help to avoid
running pipelines again.
- When working locally. 
    We could want to push to a remote branch, in this case we add only the last commit. (or the ones selected)
    In case that we push to a new branch we could keep the first and last commit. (or the ones selected)
    If we want to keep track of all the experiments locally we could push all the db. (or the selected only)
