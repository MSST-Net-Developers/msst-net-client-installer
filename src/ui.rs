use std::io::{self, Write};

pub fn prompt_yn(question: &str) -> bool {
    loop {
        print!("{} [y/n]: ", question);
        io::stdout().flush().unwrap();
        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            return false;
        }
        match line.trim().to_lowercase().as_str() {
            "y" | "yes" => return true,
            "n" | "no" => return false,
            _ => println!("请输入 y 或 n。"),
        }
    }
}

pub fn prompt_select(prompt: &str, options: &[&str]) -> usize {
    println!("{}", prompt);
    for (i, opt) in options.iter().enumerate() {
        println!("  [{}] {}", i + 1, opt);
    }
    loop {
        print!("请选择 [1-{}]：", options.len());
        io::stdout().flush().unwrap();
        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            continue;
        }
        if let Ok(n) = line.trim().parse::<usize>() {
            if n >= 1 && n <= options.len() {
                return n - 1;
            }
        }
        println!("输入无效，请输入 1 到 {} 之间的数字。", options.len());
    }
}