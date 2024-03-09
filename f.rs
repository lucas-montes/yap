/// TODO: with a similar structure allow to manage the files from a dir point of view
/// meaning that instead of passing a file we pass a dir and we fetch all the db/files
/// under the fiven dir

fn start_track_file(file: PathBuf, branch: String) {
    // branch by default would be master
    create_file_snapshot(file, branch);
}

fn create_branch()

fn create_file_snapshot(file: pathbuf, branch: string) {
    let timestamp = Local::now().timestamp().to_string();
    let file_branch_db = create_local_db(format!(".yap/logbooks/{file}.db"));
    let local_path = format!(".yap/versions/{file}/{branch}/{timestamp}");
    create_local_copy(file, local_path);
    file_branch_db.insert(
        "INSERT INTO {branch} (file, size, timestamp) VALUES (?1, ?2, ?3)",
        params![file, file.size(), timestamp],
    );
    let logbook = connect_to_local_logbook();
    logbook.insert(
        "INSERT INTO events (file, branch, timestamp, event) VALUES (?1, ?2, ?3, ?4)",
        params![file, branch, timestamp, "ADD"],
    );
}

fn commit_file_changes(file: pathbuf, branch: string,diff_config: DiffConfig){
    let timestamp = Local::now().timestamp().to_string();
    create_file_snapshot(file, branch);
    let file_branch_db = get_local_db(format!(".yap/logbooks/{file}.db")).branches().get(branch);
    let previous_snapshot = file_branch_db.snapshots().sort().lt(timestamp).first();
    compare_versions(file, preiovs_snapshot.local_path(), diff_config);
    //TODO: add a commit. this strutc will hold the diff path file, results, msg, etc...
    let logbook = connect_to_local_logbook();
    logbook.insert(
        "INSERT INTO events (file, branch, timestamp, event) VALUES (?1, ?2, ?3, ?4)",
        params![file, branch, timestamp, "COMMIT"],
    );
}

fn push(file: PathBuf, branch: String, push_config: PushConfig){
    if push_config.all{
        create_replica_of_local_db();
        push_all_files();
    } else {    
    let file_branch_db = get_local_db(format!(".yap/logbooks/{file}.db")).branches().get(branch);
    let local_snapshots = file_branch_db.snapshots().sort().compress();
    let remote_file_branch_db = get_or_create_remote_db(format!(".yap/logbooks/{file}"));
    local_snapshots.iter().map(|f|f.push()).for_each(remote_file_db.branches().get(branch).insert);
        //insert diffs too
    let logbook = connect_to_local_logbook();
    logbook.insert(
        "INSERT INTO events (file, branch, timestamp, event) VALUES (?1, ?2, ?3, ?4)",
        params![file, branch, timestamp, "PUSH"],
    );
    }
}


fn pull(file: PathBuf, branch: String, pull_config: PullConfig){
    let remote_file_branch_db = get_remote_db(format!(".yap/logbooks/{file}")).branches().get(branch);
    // TODO: use a match
    if pull_config.all{
        let files = remote_file_branch_db.pull().all();
        files.move_last_to(pull_config.path_to);
    } else if pull_config.snapshot {
        let file = remote_file_branch_db.pull().get(pull_config.snapshot);
        file.move(pull_config.path_to);
    } else if pull_config.filter {
        let files =remote_file_branch_db.pull().filter(pull_config.filter);
        files.move_last_to(pull_config.path_to);
    } else {
        let file = remote_file_branch_db.pull().last();
        file.move(pull_config.path_to);
    }
}
