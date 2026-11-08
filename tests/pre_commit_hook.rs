//! pre-commit フック(`.agent/hooks/pre-commit`)の確認。
//!
//! 使い捨ての Git リポジトリを作り、フック本体は、このリポジトリのものをそのまま動かす。
//! フックが検査するのは、作業ツリーではなく、これからコミットされる(ステージ済みの)内容。
#![cfg(unix)]

use std::{
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::{Command, Output},
};

const FORMATTED: &str = "fn main() {\n    println!(\"x\");\n}\n";
const UNFORMATTED: &str = "fn main(){println!(\"x\");}\n";
const MANIFEST: &str = "[package]\nname = \"demo\"\nversion = \"VERSION\"\nedition = \"2024\"\n";

fn run(dir: &Path, program: &str, args: &[&str], env: &[(&str, &Path)]) -> Output {
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir(dir)
        .env_remove("GIT_DIR")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_WORK_TREE")
        .env_remove("CARGO_TARGET_DIR");
    for (key, value) in env {
        cmd.env(key, value);
    }
    cmd.output().expect("コマンドを起動できる")
}

fn ok(dir: &Path, program: &str, args: &[&str]) {
    let out = run(dir, program, args, &[]);
    assert!(
        out.status.success(),
        "{program} {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn write(dir: &Path, path: &str, content: &str) {
    fs::write(dir.join(path), content).expect("書き込める");
}

fn manifest(version: &str) -> String {
    MANIFEST.replace("VERSION", version)
}

fn executable(path: &Path) {
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("権限を変えられる");
}

/// 整形済みの `main.rs` だけを持つ、コミット済みの使い捨てリポジトリ。
fn new_repo(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("pre-commit-hook-{name}"));
    let _ = fs::remove_dir_all(&dir);
    for sub in ["src", ".agent/hooks", ".agent/architecture"] {
        fs::create_dir_all(dir.join(sub)).expect("ディレクトリを作れる");
    }
    write(&dir, "Cargo.toml", &manifest("0.1.0"));
    write(&dir, "src/main.rs", FORMATTED);
    write(&dir, ".gitignore", "/target\n");
    let hook = Path::new(env!("CARGO_MANIFEST_DIR")).join(".agent/hooks/pre-commit");
    fs::copy(hook, dir.join(".agent/hooks/pre-commit")).expect("フックをコピーできる");
    // 依存検査はこのテストの対象外。常に成功するもので置き換える
    write(
        &dir,
        ".agent/architecture/check.sh",
        "#!/usr/bin/env bash\nexit 0\n",
    );
    executable(&dir.join(".agent/architecture/check.sh"));

    ok(&dir, "git", &["init", "-q"]);
    ok(&dir, "git", &["config", "user.email", "test@example.com"]);
    ok(&dir, "git", &["config", "user.name", "test"]);
    ok(&dir, "cargo", &["generate-lockfile", "--quiet"]);
    ok(&dir, "git", &["add", "-A"]);
    ok(&dir, "git", &["commit", "-q", "-m", "init"]);
    dir
}

fn hook(dir: &Path, env: &[(&str, &Path)]) -> Output {
    run(dir, "bash", &[".agent/hooks/pre-commit"], env)
}

fn text(out: &Output) -> String {
    format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    )
}

#[test]
fn it_passes_when_everything_staged_is_fine() {
    let dir = new_repo("passes");
    let out = hook(&dir, &[]);
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn it_checks_the_staged_content_not_the_working_tree() {
    let dir = new_repo("staged");
    write(&dir, "src/main.rs", UNFORMATTED);
    ok(&dir, "git", &["add", "src/main.rs"]);
    // 作業ツリーだけを直す。コミットされるのは、整形されていない方
    write(&dir, "src/main.rs", FORMATTED);

    let out = hook(&dir, &[]);
    assert!(
        !out.status.success(),
        "整形されていない内容がコミットされてしまう"
    );
}

#[test]
fn it_ignores_breakage_that_is_not_staged() {
    let dir = new_repo("unstaged");
    write(
        &dir,
        "src/main.rs",
        "fn main() {\n    println!(\"y\");\n}\n",
    );
    ok(&dir, "git", &["add", "src/main.rs"]);
    // 壊れているが、ステージしていない。コミットには入らない
    write(&dir, "src/main.rs", "fn main( {\n");

    let out = hook(&dir, &[]);
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn it_does_not_touch_the_working_tree() {
    let dir = new_repo("untouched");
    write(
        &dir,
        "src/main.rs",
        "fn main() {\n    println!(\"y\");\n}\n",
    );
    ok(&dir, "git", &["add", "src/main.rs"]);
    write(&dir, "src/main.rs", "// 作業中の変更\n");
    write(&dir, "notes.txt", "未追跡のファイル\n");

    let before = run(&dir, "git", &["status", "--porcelain"], &[]).stdout;
    let main_before = fs::read_to_string(dir.join("src/main.rs")).unwrap();
    let _ = hook(&dir, &[]);

    assert_eq!(
        run(&dir, "git", &["status", "--porcelain"], &[]).stdout,
        before
    );
    assert_eq!(
        fs::read_to_string(dir.join("src/main.rs")).unwrap(),
        main_before
    );
    assert_eq!(
        fs::read_to_string(dir.join("notes.txt")).unwrap(),
        "未追跡のファイル\n"
    );
    assert!(
        !dir.join(".git/pre-commit-snapshot").exists(),
        "取り出したディレクトリを残さない"
    );
}

#[test]
fn it_fails_when_the_staged_lock_file_does_not_match_the_manifest() {
    let dir = new_repo("lock");
    write(&dir, "Cargo.toml", &manifest("0.1.1"));
    ok(&dir, "git", &["add", "Cargo.toml"]); // Cargo.lock は、古いままステージされている

    let out = hook(&dir, &[]);
    assert!(!out.status.success(), "{}", text(&out));
}

#[test]
fn it_follows_the_index_that_git_gives_the_hook() {
    // `git commit <paths>` は、フックに専用のインデックスを渡す。本物のインデックスではなく、そちらを見る
    let dir = new_repo("index");
    write(&dir, "src/main.rs", UNFORMATTED);
    ok(&dir, "git", &["add", "src/main.rs"]); // 本物のインデックス: 整形されていない

    let temporary = dir.join(".git/temporary-index");
    let env = [("GIT_INDEX_FILE", temporary.as_path())];
    let read_tree = run(&dir, "git", &["read-tree", "HEAD"], &env);
    assert!(read_tree.status.success());
    write(&dir, "src/main.rs", FORMATTED);
    let update = run(&dir, "git", &["update-index", "--add", "src/main.rs"], &env);
    assert!(update.status.success());

    let out = hook(&dir, &env);
    assert!(out.status.success(), "{}", text(&out));
}
