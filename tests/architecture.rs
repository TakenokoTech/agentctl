//! 層間の依存ルールの検査。ルールは `.agent/architecture/SKILL.md` を参照。
//!
//! `src/**/*.rs` を `syn` で解析し、`use` と、式や型の中のパスを集める。`crate::`、`self::`、
//! `super::` は、そのファイルが定義するモジュールを基準に解決する。複数行の `use`、
//! 入れ子のグループ、`super::` などの相対パスも、文字列の一致ではなく構文として扱う。
use std::{
    fs,
    path::{Path, PathBuf},
};

use syn::{ItemUse, UseTree, visit::Visit};

/// 層 -> 依存してはいけない層
const RULES: &[(&str, &[&str])] = &[
    ("domain", &["application", "infrastructure", "presentation"]),
    ("application", &["infrastructure", "presentation"]),
    ("infrastructure", &["presentation"]),
    ("presentation", &["infrastructure"]),
];

/// `module` で書かれたパスを、クレートのルートからの絶対パスにする(ルートは空)。
/// 外部クレートや、その場の名前から始まるパスは `None`。
fn resolve(module: &[String], path: &[String]) -> Option<Vec<String>> {
    let (first, rest) = path.split_first()?;
    let mut base = match first.as_str() {
        "crate" => Vec::new(),
        "self" => module.to_vec(),
        "super" => {
            let mut parent = module.to_vec();
            parent.pop()?;
            parent
        }
        _ => return None,
    };
    for segment in rest {
        match segment.as_str() {
            "super" => {
                base.pop()?;
            }
            "self" => {}
            other => base.push(other.to_owned()),
        }
    }
    Some(base)
}

/// `use` の木を、1本ずつのパスに展開する。
fn flatten(tree: &UseTree, prefix: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
    match tree {
        UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            flatten(&p.tree, prefix, out);
            prefix.pop();
        }
        UseTree::Name(n) => {
            prefix.push(n.ident.to_string());
            out.push(prefix.clone());
            prefix.pop();
        }
        UseTree::Rename(r) => {
            prefix.push(r.ident.to_string());
            out.push(prefix.clone());
            prefix.pop();
        }
        UseTree::Glob(_) => out.push(prefix.clone()),
        UseTree::Group(g) => {
            for item in &g.items {
                flatten(item, prefix, out);
            }
        }
    }
}

struct Finder<'a> {
    module: &'a [String],
    forbidden: &'a [&'a str],
    found: Vec<String>,
}

impl Finder<'_> {
    fn check(&mut self, path: &[String]) {
        if let Some(resolved) = resolve(self.module, path)
            && let Some(layer) = resolved.first()
            && self.forbidden.contains(&layer.as_str())
        {
            self.found.push(path.join("::"));
        }
    }
}

impl<'ast> Visit<'ast> for Finder<'_> {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        let mut paths = Vec::new();
        flatten(&item.tree, &mut Vec::new(), &mut paths);
        for path in paths {
            self.check(&path);
        }
    }

    fn visit_path(&mut self, path: &'ast syn::Path) {
        let segments: Vec<String> = path.segments.iter().map(|s| s.ident.to_string()).collect();
        self.check(&segments);
        syn::visit::visit_path(self, path);
    }
}

/// `module` を定義するファイルの `source` から、層のルールに反するパスを集める。
fn violations(module: &[String], source: &str) -> Vec<String> {
    let Some(layer) = module.first() else {
        return Vec::new();
    };
    let Some((_, forbidden)) = RULES.iter().find(|(name, _)| name == layer) else {
        return Vec::new();
    };
    let file = syn::parse_file(source).expect("ソースを構文解析できる");
    let mut finder = Finder {
        module,
        forbidden,
        found: Vec::new(),
    };
    finder.visit_file(&file);
    finder.found
}

/// `src/domain/a2a/tests.rs` -> `["domain", "a2a", "tests"]`。`src/main.rs` はクレートのルート。
fn module_of(src: &Path, file: &Path) -> Vec<String> {
    let relative = file.strip_prefix(src).expect("src の下").with_extension("");
    let mut parts: Vec<String> = relative
        .iter()
        .map(|part| part.to_string_lossy().into_owned())
        .collect();
    if parts == ["main"] {
        parts.clear();
    }
    parts
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("ディレクトリを読める") {
        let path = entry.expect("エントリを読める").path();
        if path.is_dir() {
            // src/bin/ は別のプログラムで、層構成の対象外
            if path.file_name().is_some_and(|name| name == "bin") {
                continue;
            }
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_codebase_respects_the_layer_rules() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    rust_files(&src, &mut files);
    let mut problems = Vec::new();
    for file in files {
        let module = module_of(&src, &file);
        let source = fs::read_to_string(&file).expect("ソースを読める");
        for path in violations(&module, &source) {
            problems.push(format!("{}: {path}", file.display()));
        }
    }
    assert!(
        problems.is_empty(),
        "layer violations:\n{}",
        problems.join("\n")
    );
}

// ---------- 検査そのものの確認 ----------

fn module(path: &str) -> Vec<String> {
    path.split("::").map(str::to_owned).collect()
}

#[test]
fn detects_a_relative_super_path_from_a_layer_root_file() {
    assert_eq!(
        violations(&module("domain"), "use super::infrastructure::Foo;"),
        ["super::infrastructure::Foo"]
    );
}

#[test]
fn detects_every_forbidden_member_of_a_multiline_group() {
    let source = "use crate::{\n    application,\n    infrastructure,\n};";
    assert_eq!(violations(&module("domain"), source).len(), 2);
}

#[test]
fn detects_a_forbidden_layer_after_a_nested_group() {
    let source = "use crate::{domain::{a, b}, infrastructure::c};";
    assert_eq!(violations(&module("application"), source).len(), 1);
}

#[test]
fn detects_a_path_split_across_lines() {
    let source = "use crate::\n    presentation::Bar;";
    assert_eq!(violations(&module("domain"), source).len(), 1);
}

#[test]
fn detects_self_super_and_glob_imports() {
    let via_self_super = "use self::super::infrastructure::Baz;";
    assert_eq!(violations(&module("domain"), via_self_super).len(), 1);
    let glob = "use crate::presentation::*;";
    assert_eq!(violations(&module("domain"), glob).len(), 1);
}

#[test]
fn detects_a_path_used_without_a_use_declaration() {
    let source = "fn f() { let _ = crate::infrastructure::config::load; }";
    assert_eq!(violations(&module("application"), source).len(), 1);
}

#[test]
fn resolves_super_relative_to_the_files_own_module() {
    // domain/a2a.rs の `super::infrastructure` は `domain::infrastructure` で、層ではない
    assert!(violations(&module("domain::a2a"), "use super::infrastructure::Foo;").is_empty());
    // domain/a2a/tests.rs から3つ上がるとクレートのルートなので、こちらは層そのもの
    let source = "use super::super::super::infrastructure::Foo;";
    assert_eq!(violations(&module("domain::a2a::tests"), source).len(), 1);
}

#[test]
fn allows_dependencies_that_the_rules_permit() {
    let ok = [
        (
            "application::a2a",
            "use crate::domain::a2a::Foo; use super::*;",
        ),
        (
            "presentation::cli",
            "use crate::application::mcp; use anyhow::Result;",
        ),
        (
            "infrastructure::mcp",
            "use crate::domain::mcp::McpSession; use std::fs;",
        ),
    ];
    for (path, source) in ok {
        assert!(violations(&module(path), source).is_empty(), "{path}");
    }
}

#[test]
fn files_outside_the_layers_are_not_checked() {
    // main.rs(ルート)は各層を組み立てる側なので、どの層にも依存してよい
    assert!(violations(&[], "use crate::infrastructure::a2a::A2aRustConnector;").is_empty());
}
