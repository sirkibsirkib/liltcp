
extern crate clap;
use clap::App;
use std::thread;
use std::net::SocketAddr;
use std::io::{Read, Write};
use std::fmt;
use std::collections::HashSet;
extern crate base64;
extern crate hex;


#[derive(Copy, Clone)]
enum Setting {
    Hex, Utf8, Base64
}
impl fmt::Display for Setting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let s = match self {
            &Setting::Hex => "hex",
            &Setting::Base64 => "base 64",
            &Setting::Utf8 => "utf-8",
        };
        write!(f, "{}", s)
    }
}
fn main() {
    let matches = App::new("LilTcp")
            .version("1.0")
            .author("C. Esterhuyse <christopher.esterhuyse@gmail.com>")
            .about("Lets you manually, interactively exchange bytes with a TcpStream. Can deal with utf-8 `u`, hexadecimal `h` and base-64 `b`.")
            .args_from_usage("-s, --server 'Bind and assume the roll of server until you accept the first connection.'
                             -i, --input=[ENCODING] 'Starting input encoding from {u, h, b}. Can be changed at runtime.'
                             -o, --output=[ENCODING_STR] 'Starting output incoding set. Eg `ub` for unicode and base64' 
                             <ip> 'Sets the bind/connect addr'")
            .get_matches();
    let ip = matches.value_of("ip").unwrap();
    let setting = match matches.value_of("input") {
        Some("h") => Setting::Hex,
        Some("b") => Setting::Base64,
        None |
        Some("u") => Setting::Utf8,
        _ => {
            println!(">> Malformed `input` choice. Run again with --help.");
            return;
        },
    };
    if let Some(s) = matches.value_of("output") {
        if !try_set_output_encoding(s) {
            println!(">> Malformed `output` choice. Run again with --help.");
        }
    }
    if let Ok(addr) = ip.parse::<SocketAddr>() {
        match matches.occurrences_of("server") {
            0 => client(addr, setting),
            1 | _ => server(addr, setting),
        }
    } else {
        println!(">> Couldn't parse ip string `{}`. Good example: `127.0.0.1:8000`", ip);
    }
}


fn client(addr: SocketAddr, setting: Setting) {
    println!(">> Connecting to {:?}", &addr);
    let sock = std::net::TcpStream::connect(&addr).expect("Failed to connect");
    println!(">> Connected!");
    go(sock, setting);
}

fn server(addr: SocketAddr, setting: Setting) {
    println!(">> Binding to {:?}", &addr);
    let x = std::net::TcpListener::bind(&addr).expect("failed to bind");
    println!(">> Listening...");
    let (sock, addr) = x.accept().unwrap();
    println!(">> Connection from {:?}!", addr);
    go(sock, setting);
}


static mut SHOW_HEX: bool = true;
static mut SHOW_UTF8: bool = true;
static mut SHOW_B64: bool = true;

fn print_output_set() {
    let mut set = HashSet::new();
    unsafe {
        if SHOW_UTF8 { set.insert('u'); }
        if SHOW_HEX { set.insert('h'); }
        if SHOW_B64 { set.insert('b'); }
    }
    println!(">> Output set is {:?}", set);
}

fn go(mut stream: std::net::TcpStream, mut setting: Setting) {
    stream.set_nodelay(true).unwrap();
    let mut stream2 = stream.try_clone().unwrap();
    stream2.set_nodelay(true).unwrap();
    println!(">> Enter just `/?` for a print of what commands exist");
    print_output_set();

    thread::spawn(move || {
        //OUTGOING thread
        let mut key_buffer = String::new();
        println!(">> Listening for {} input.", setting);
        loop {
            key_buffer.clear();
            let mut in_bytes = std::io::stdin().read_line(&mut key_buffer).expect("reader died");
            if key_buffer[..in_bytes].chars().last() == Some('\n') {
                in_bytes -= 1;
            }
            if key_buffer[..in_bytes].chars().last() == Some('\r') {
                in_bytes -= 1;
            }
            if &key_buffer[0..in_bytes] == "/?" {
                println!(">> Available commands: (preceded by '/')");
                println!("   * ('?') shows this help.");
                println!("   * ('i' + [encoding_str]) where encoding_str is a string containing");
                println!("     chars `u`, `h`, `b` for utf-8, hexadecimal and base 64 respectively");
                println!("   * ('o' + [encoding]) where encoding is in {{`u`, `h`, `b`}}.");
                println!("     for utf-8, hexadecimal and base 64 respectively");
                println!("   * ('!' + [input]) displays the given input in your output encodings without sending.");
            } else if key_buffer.chars().nth(0) == Some('!') {
                print_out(& key_buffer[1..in_bytes].as_bytes());
            }else if key_buffer.chars().nth(0) == Some('/') {
                if key_buffer[0..in_bytes].chars().nth(1) == Some('/') {
                    send_text(&key_buffer[1..in_bytes], &mut stream2, setting);
                } else {
                    //command 
                    match key_buffer[0..in_bytes].chars().nth(1) {
                        Some('o') => {
                            if try_set_output_encoding(& key_buffer[2..in_bytes]) {
                                print_output_set();
                            }  else {
                                prompt_help();
                            }
                        }
                        Some('i') => {
                            match &key_buffer[2..in_bytes] {
                                "u" => {
                                    setting = Setting::Utf8;
                                    println!("Parsing input as utf-8 string.");
                                },
                                "h" => {
                                    setting = Setting::Hex;
                                    println!("Parsing input as hexadecimal string.");
                                },
                                "b" => {
                                    setting = Setting::Base64;
                                    println!("Parsing input as base 64 string.");
                                },
                                _ => prompt_help(),
                            }
                        },
                        _ => prompt_help(),
                    }
                    continue;
                }
            } else {
                send_text(&key_buffer[..in_bytes], &mut stream2, setting);
            }
        }
    });

    let mut buffer = [0u8; 80];
    loop {
        match stream.read(&mut buffer) {
            Ok(bytes_read) => print_out(&buffer[..bytes_read]),
            Err(e) => {
                println!("{:?}", e);
                return;
            }
        }
    }
}

fn print_out(bytes: &[u8]) {
    unsafe {
        if SHOW_UTF8 {
            if let Ok(s) = std::str::from_utf8(bytes) {
                println!("[str] `{}`", s);
            }
        }
        if SHOW_B64 {
            if let Ok(s) = std::str::from_utf8(bytes) {
                println!("[b64]  {}", base64::encode(s));
            }
        }
        if SHOW_HEX{
            if let Ok(s) = std::str::from_utf8(bytes) {
                println!("[hex]  {}", hex::encode(s));
            }
        }
        println!();
    }
}

fn prompt_help() {
    println!(">> Malformed command. Enter `/?` for help");
}

fn try_set_output_encoding(s: &str) -> bool {
    let mut u = false;
    let mut h = false;
    let mut b = false;
    for c in s.chars() {
        match c {
            'u' => u = true,
            'h' => h = true,
            'b' => b = true,
            _ => return false,
        }
    }
    unsafe {
        SHOW_UTF8 = u;
        SHOW_HEX = h;
        SHOW_B64 = b;
    }
    true
}

fn send_text(text: &str, stream: &mut std::net::TcpStream, setting: Setting) {
    match setting {
        Setting::Utf8 => {
            stream.write(&text.as_bytes()).expect("writing went bad");
        },
        Setting::Base64 => {
            if let Ok(b) = base64::decode(&text) {
                stream.write(&b).expect("writing went bad");
            } else {
                println!(">> Failed to encode as b64");
            }
        },
        Setting::Hex => {
            if let Ok(b) = hex::decode(&text) {
                stream.write(&b).expect("writing went bad");
            } else {
                println!(">> Failed to encode as hex");
            }
        },
    }
}