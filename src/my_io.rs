use std::process::Command;
use std::env;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn recreate_files() {
    let kinds = include_str!("../prolog/kinds.pl");
    let type_checker = include_str!("../prolog/type_checker.pl");
    let type_comparison = include_str!("../prolog/type_comparison.pl");
    let type_context = include_str!("../prolog/type_context.pl");
    let type_module = include_str!("../prolog/type_module.pl");
    let type_printer = include_str!("../prolog/type_printer.pl");
    let unification = include_str!("../prolog/unification.pl");

    let prolog_files = [kinds, type_checker, type_comparison, type_context, type_module, type_printer, unification];
    ["kinds.pl", "type_checker.pl", "type_comparison.pl", "type_context.pl", "type_module.pl", "type_printer.pl", "unification.pl"].iter().zip(prolog_files.iter()).for_each(|(name, content)| {
        let path = Path::new(&name);
        let mut file = File::create(path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
    });
}

fn delete_file(file_path: &str) {
    let path = Path::new(file_path);
    let _ = fs::remove_file(path);
}

pub fn delete_files() {
    ["kinds.pl", "type_checker.pl", "type_comparison.pl", "type_context.pl", "type_module.pl", "type_printer.pl", "unification.pl"].iter().for_each(|file| delete_file(file))
}

pub fn read_file() -> String {
    let args: Vec<String> = env::args().collect();
    if cfg!(windows){
        let file = args[1].replace("\\", r"\");
        fs::read_to_string(&file).unwrap()
    } else {
        let file = &args[1];
        fs::read_to_string(&file).unwrap()
    }
}

pub fn read_file_from_name(name: &str) -> String {
    let file = format!("{}.ty", name);
    fs::read_to_string(&file).unwrap()
}

pub fn execute() -> () {
    println!("Type checking: ");
    let output = Command::new("swipl")
        .arg("-s")
        .arg("adt.pl")
        .arg("-g")
        .arg("main")
        .arg("-t")
        .arg("halt")
        .output()
        .expect("Échec lors de l'exécution de la commande");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{}", stdout);

    println!("");

    println!("Execution: ");
    let output = Command::new("Rscript")
        .arg("app.R")
        .output()
        .expect("Échec lors de l'exécution de la commande");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{}", stdout);
}

pub fn write_adt(s: &str) {
    let mut file = File::create("adt.pl").unwrap();
    let import1 = ":- use_module(type_checker).";
    let import2 = ":- use_module(type_printer).";
    let intro = "main :-";
    let begin_typing = "typing(context([], []), ";
    let stds = include_str!("../configs/std.pl");
    let new_s = s.replacen("sequence([", &("sequence([\n".to_string() + stds + ","), 1);
    let end_typing = ", T),";
    let write = "type_printer:pretty_print(T)";
    file.write_all(format!("{}\n{}\n\n{}\n{}{}{}\n\t{}.",
                           import1, import2, intro, begin_typing, new_s, end_typing, write)
                   .as_bytes()).unwrap();
}
