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
    // 取り出し先の置き場を、このテスト専用にして、後始末を確かめる
    let scratch = dir.with_file_name("pre-commit-hook-untouched-scratch");
    let _ = fs::remove_dir_all(&scratch);
    fs::create_dir_all(&scratch).expect("ディレクトリを作れる");
    let _ = hook(&dir, &[("TMPDIR", scratch.as_path())]);

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
    assert_eq!(
        fs::read_dir(&scratch).unwrap().count(),
        0,
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
fn it_fails_on_a_stale_lock_file_even_if_the_architecture_check_would_update_it() {
    // 依存検査は `--locked` なしで cargo を動かす。Cargo.lock が古くても、それが書き換えて直してしまう
    let dir = new_repo("lock-updated-by-check");
    write(
        &dir,
        ".agent/architecture/check.sh",
        "#!/usr/bin/env bash\ncargo metadata --format-version 1 >/dev/null\n",
    );
    executable(&dir.join(".agent/architecture/check.sh"));
    write(&dir, "Cargo.toml", &manifest("0.1.1"));
    ok(&dir, "git", &["add", "-A"]); // Cargo.lock は、古いままステージされている

    let out = hook(&dir, &[]);
    assert!(
        !out.status.success(),
        "古い Cargo.lock がコミットされてしまう: {}",
        text(&out)
    );
}

#[test]
fn it_checks_files_marked_export_ignore() {
    // `git archive` は export-ignore のファイルを取り出さない。コミットされる内容は、そのまま検査する
    let dir = new_repo("export-ignore");
    write(&dir, ".gitattributes", "src/main.rs export-ignore\n");
    ok(&dir, "git", &["add", ".gitattributes"]);

    let out = hook(&dir, &[]);
    assert!(out.status.success(), "{}", text(&out));
}

#[test]
fn it_lets_two_hooks_run_in_the_same_worktree_at_once() {
    // Git は、フックの実行中にインデックスをロックしない。同じワークツリーで、フックが重なることがある
    let dir = new_repo("concurrent");
    let sync = dir.join("sync");
    fs::create_dir_all(&sync).expect("ディレクトリを作れる");
    // 先に着いた方(`mkdir` に成功した方)は、もう一方が終わるまで、取り出し先の中で待つ。
    // 待ち合わせはファイルで行うので、時間に頼らずに、確実に重なる
    let script = format!(
        "#!/usr/bin/env bash\n\
         sync='{sync}'\n\
         if mkdir \"$sync/first\" 2>/dev/null; then\n\
         \x20 for _ in $(seq 300); do [ -e \"$sync/second-done\" ] && break; sleep 0.1; done\n\
         \x20 [ -e \"$sync/second-done\" ] && test -f src/main.rs\n\
         else\n\
         \x20 test -f src/main.rs && touch \"$sync/second-done\"\n\
         fi\n",
        sync = sync.display()
    );
    write(&dir, ".agent/architecture/check.sh", &script);
    executable(&dir.join(".agent/architecture/check.sh"));
    ok(&dir, "git", &["add", ".agent/architecture/check.sh"]); // フックが実行するのは、ステージ済みの方

    let first = std::thread::spawn({
        let dir = dir.clone();
        move || hook(&dir, &[])
    });
    for _ in 0..600 {
        if sync.join("first").exists() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert!(sync.join("first").exists(), "1つ目のフックが検査に着かない");
    let second = hook(&dir, &[]);
    let first = first.join().expect("スレッドが終わる");

    assert!(first.status.success(), "1つ目: {}", text(&first));
    assert!(second.status.success(), "2つ目: {}", text(&second));
}

#[test]
fn it_checks_files_that_a_sparse_checkout_leaves_out() {
    // skip-worktree のファイルは、作業ツリーに無いが、コミットには含まれる
    let dir = new_repo("sparse");
    ok(
        &dir,
        "git",
        &["update-index", "--skip-worktree", "src/main.rs"],
    );
    fs::remove_file(dir.join("src/main.rs")).expect("消せる");
    write(&dir, "notes.txt", "メモ\n");
    ok(&dir, "git", &["add", "notes.txt"]);

    let out = hook(&dir, &[]);
    assert!(out.status.success(), "{}", text(&out));
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
