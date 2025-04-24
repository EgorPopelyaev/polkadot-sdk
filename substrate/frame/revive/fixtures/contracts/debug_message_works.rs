// This file is part of Substrate.

// Copyright (C) Parity Technologies (UK) Ltd.
// SPDX-License-Identifier: Apache-2.0

// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// 	http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Emit a "Hello World!" debug message.
#![no_std]
#![no_main]

extern crate common;
use uapi::{HostFn, HostFnImpl as api};

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn deploy() {}

<<<<<<< HEAD:substrate/frame/revive/fixtures/contracts/debug_message_works.rs
#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn call() {
	api::debug_message(b"Hello World!").unwrap();
=======
	#[pallet::storage]
	pub type MyStorage<T> = StorageValue<_, u32>;

	#[pallet::view_functions]
	impl<T: Config> Pallet<T> {
		pub fn get_value() {
			MyStorage::<T>::set(0);
		}
	}
>>>>>>> 07827930 (Use original pr name in prdoc check (#60)):substrate/frame/support/test/tests/pallet_ui/view_function_no_return.rs
}
