#[cfg(test)]
#[macro_use]
pub mod macros {
	macro_rules! vec_of_strings {
		($($x:expr),*) => (vec![$($x.to_string()),*]);
	}
}

pub mod config;
pub mod kubeconfig;
pub mod proxy;
pub mod spawn;
pub mod state;
