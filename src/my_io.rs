use std::process::Command;
use std::env;
use std::fs;
use std::path::PathBuf;


fn get_os_file(file: &str) -> String {
    if cfg!(windows){
       file.replace("\\", r"\") 
    } else {
       file.to_string()
    }
}

pub fn read_file(path: &PathBuf) -> String {
    let args: Vec<String> = env::args().collect();
    let file = get_os_file(path.to_str().unwrap());
    fs::read_to_string(&file).unwrap()
}

pub fn read_file_from_name(name: &str) -> String {
    let file = get_os_file(&format!("{}.ty", name));
    fs::read_to_string(&file).expect(&format!("Can't Read file {}", name))
}

pub fn execute_r() -> () {
    println!("Execution: ");
    let output = Command::new("Rscript")
        .arg(get_os_file("app.R"))
        .output()
        .expect("Échec lors de l'exécution de la commande");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{}", stdout);
}

pub fn execute_r_with_path(execution_path: &PathBuf) -> () {
    let output = Command::new("Rscript")
        .current_dir(execution_path)
        .arg(get_os_file("main.R"))
        .output()
        .expect("Échec lors de l'exécution de la commande");

    let stdout = String::from_utf8_lossy(&output.stdout);
    println!("{}", stdout);
}

pub fn execute_wasm() -> () {
    println!("Compilation TypeScript: ");
    
    // Compiler le fichier TypeScript en JavaScript
    let tsc_output = Command::new("tsc")
        .arg("app.ts")
        .output()
        .expect("Échec lors de la compilation TypeScript");
    
    if !tsc_output.status.success() {
        let stderr = String::from_utf8_lossy(&tsc_output.stderr);
        println!("Erreur de compilation TypeScript: {}", stderr);
        return;
    }
    
    println!("Exécution JavaScript: ");
    
    // Exécuter le fichier JavaScript compilé
    let node_output = Command::new("node")
        .arg("app.js")
        .output()
        .expect("Échec lors de l'exécution de Node.js");
    
    let stdout = String::from_utf8_lossy(&node_output.stdout);
    let stderr = String::from_utf8_lossy(&node_output.stderr);
    
    if !node_output.status.success() {
        println!("Erreur d'exécution JavaScript: {}", stderr);
    } else {
        println!("{}", stdout);
        if !stderr.is_empty() {
            println!("Avertissements: {}", stderr);
        }
    }
}
