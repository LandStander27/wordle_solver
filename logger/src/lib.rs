#![allow(clippy::needless_return)]

use syn::{parse_macro_input, ItemFn};

// macro_rules! log {
// 	($x:expr) => {
// 		{
// 			// println!("Starting function: {}", stringify!($x).chars().take_while(|&c: &char| c != '(' ).collect::<String>());

// 			let start = std::time::Instant::now();

// 			// let dt = std::time::SystemTime::now();
// 			// dt.format("%H:%M:%S").unwrap();

// 			let mut dt = chrono::Local::now();
// 			let name = stringify!($x).chars().take_while(|&c: &char| c != '(' ).collect::<String>();
// 			println!("\x1b[32m[{}] \x1b[96m{}\x1b[0m", dt.format("%H:%M:%S"), name);

// 			let ret = $x;

// 			dt = chrono::Local::now();
// 			println!("\x1b[32m[{}] \x1b[96m{} \x1b[92mCompleted in {}ms\x1b[0m", dt.format("%H:%M:%S"), name, (start.elapsed().as_secs_f32()*10000.0).round()/100.0);
// 			// println!("{}ms", start.elapsed().as_millis());
// 			ret
// 		}
// 	};
// }

// macro_rules! a {
//     ($($tt:tt)*) => {
//         {
// 			println!("blah blah: {}", stringify!($($tt)*));
// 			$($tt)*
// 		}
//     };
// }

#[proc_macro_attribute]
pub fn logger(_attr: proc_macro::TokenStream, item: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(item as ItemFn);

	let ItemFn { attrs, vis, sig, block  } = input;
	let statements = &block.stmts;

	let name = sig.ident.to_string();

	let out = quote::quote!(
		#[allow(unreachable_code)]
		#(#attrs)* #vis #sig {

			#[cfg(windows)]
			{
				use windows_sys::Win32::System::Console::{
					GetConsoleMode, GetStdHandle, SetConsoleMode, ENABLE_VIRTUAL_TERMINAL_PROCESSING,
					STD_OUTPUT_HANDLE,
				};

				unsafe {
					let handle = GetStdHandle(STD_OUTPUT_HANDLE);
					let mut original_mode = 0;
					GetConsoleMode(handle, &mut original_mode);
					SetConsoleMode(handle, ENABLE_VIRTUAL_TERMINAL_PROCESSING | original_mode)
				}
			}

			let start = std::time::Instant::now();
			let mut dt = chrono::Local::now();
			println!("\x1b[32m[{}] \x1b[96m{}\x1b[0m", dt.format("%H:%M:%S"), #name);

			#[allow(unused_macros)]
			macro_rules! answer {
				($($tt:tt)*) => {
					{
						let dt = chrono::Local::now();
						println!("\x1b[32m[{}] \x1b[92m{}\x1b[0m", dt.format("%H:%M:%S"), format_args!($($tt)*));
					}
				};
			}

			#[allow(unused_macros)]
			macro_rules! log {
				($($tt:tt)*) => {
					{
						let dt = chrono::Local::now();
						println!("\x1b[32m[{}] \x1b[0m{}", dt.format("%H:%M:%S"), format_args!($($tt)*));
					}
				};
			}

			#[allow(unused_macros)]
			macro_rules! err {
				($($tt:tt)*) => {
					{
						let dt = chrono::Local::now();
						println!("\x1b[32m[{}] \x1b[91m{}\x1b[0m", dt.format("%H:%M:%S"), format_args!($($tt)*));
					}
				};
			}

			#[allow(unused_macros)]
			macro_rules! log_return {
				{$tt:expr} => {
					{
						dt = chrono::Local::now();
						println!("\x1b[32m[{}] \x1b[96m{} \x1b[92mCompleted in {}ms\x1b[0m", dt.format("%H:%M:%S"), #name, (start.elapsed().as_secs_f32()*100000.0).round()/100.0);
						return $tt
					}
				}
			}

			#[allow(unused_functions)]
			fn input<T>(prompt: &str) -> T where T: std::str::FromStr + std::default::Default, <T as std::str::FromStr>::Err : std::fmt::Debug {
				let dt = chrono::Local::now();
				let mut input: String = "".to_string();
				print!("\x1b[32m[{}] \x1b[0m{}", dt.format("%H:%M:%S"), prompt);
				std::io::stdout().flush().unwrap();
				std::io::stdin().read_line(&mut input).unwrap();
				return input.trim().parse::<T>().unwrap_or_default();
			}

			#(#statements)*

			{
				dt = chrono::Local::now();
				println!("\x1b[32m[{}] \x1b[96m{} \x1b[92mCompleted in {}ms\x1b[0m", dt.format("%H:%M:%S"), #name, (start.elapsed().as_secs_f32()*100000.0).round()/100.0);
			};

		}
	);



	return proc_macro::TokenStream::from(out);

}