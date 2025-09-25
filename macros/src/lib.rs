#[macro_export]
macro_rules! print_result {
    ($expression:expr) => {
        println!("{:?}: {:?}", stringify!($expression), $expression);
    };
}

#[macro_export]
macro_rules! enum_to_str {
    ($e:ident { $($variant:pat => $str:literal),* }) => {
        impl $e {
            fn as_str(&self) -> &'static str {
                match self {
                    $($variant => $str),*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! create_func {
    ($func_name:ident) => {
        fn $func_name() {
            println!("Hello, world!");
        }
    };
}

#[macro_export]
macro_rules! test_expr {
    ($left:expr; and $right:expr) => {
        println!(
            "{} and {} is {}",
            stringify!($left),
            stringify!($right),
            $left && $right
        );
    };
}

#[macro_export]
macro_rules! find_min {
        ($x:expr) => ($x);
        ($x:expr, $($y:expr),+) => {
            std::cmp::min($x, find_min!($($y),+))
        };
    }
