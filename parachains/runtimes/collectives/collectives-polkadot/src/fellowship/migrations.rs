// Copyright 2023 Parity Technologies (UK) Ltd.
// This file is part of Cumulus.

// Cumulus is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Cumulus is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Cumulus.  If not, see <http://www.gnu.org/licenses/>.

//! Migrations.

use frame_support::{pallet_prelude::*, traits::OnRuntimeUpgrade, weights::Weight};
use log;

/// Initial import of the Kusama Technical Fellowship.
pub(crate) mod import_kusama_fellowship {
	use super::*;
	use frame_support::parameter_types;
	use pallet_ranked_collective::{
		Config, IdToIndex, IndexToId, MemberCount, MemberRecord, Members,
		Pallet as RankedCollective, Rank,
	};

	const TARGET: &'static str = "runtime::migration::import_fellowship";

	parameter_types! {
		// The Fellowship addresses from Kusama state.
		pub const FellowshipAddresses: [(Rank, [u8; 32]); 45] = [
			(6, hex_literal::hex!("86b7409a11700afb027924cb40fa43889d98709ea35319d48fea85dd35004e64"),),
			(6, hex_literal::hex!("f60f9b64ebf26b9487c65ada132908745572692aef7cd9c987daf8c9c0c2ff3a"),),
			(6, hex_literal::hex!("7628a5be63c4d3c8dbb96c2904b1a9682e02831a1af836c7efc808020b92fa63"),),
			(5, hex_literal::hex!("9c84f75e0b1b92f6b003bde6212a8b2c9b776f3720f942b33fed8709f103a268"),),
			(5, hex_literal::hex!("86bdd82d59404ffe1d6120c6358c14c1bef69a013fa91771c2594f6fa310187e"),),
			(5, hex_literal::hex!("2e1884c53071526483b14004e894415f02b55fc2e2aef8e1df8ccf7ce5bd5570"),),
			(5, hex_literal::hex!("f6b21d624832094b03aa672e016462a020e217cc67b1434785b99114a2b4fa5a"),),
			(4, hex_literal::hex!("4adf51a47b72795366d52285e329229c836ea7bbfe139dbe8fa0700c4f86fc56"),),
			(4, hex_literal::hex!("d25af2fedd4eb672f218932fde44f97f10c1d7788efd0079957ffad4f186ae78"),),
			(4, hex_literal::hex!("ee9e0a9ced1d9809056b4adca8d224ea3c359ab9af1fb6986028fe816e37665a"),),
			(3, hex_literal::hex!("9ec982d60f3779d8f0043933ee6d1b2a4346df17d07e44a22a2cd91a31076352"),),
			(3, hex_literal::hex!("c4965f7fe7be8174717a24ffddf684986d122c7e293ddf875cdf9700a07b6812"),),
			(3, hex_literal::hex!("beae5bcad1a8c156291b7ddf46b38b0c61a6aaacebd57b21c75627bfe7f9ab71"),),
			(3, hex_literal::hex!("9ad97bf6cfb3fb36b24b221c27ea22cbdbfb4dd4b2c54f80d8235d490be9113b"),),
			(3, hex_literal::hex!("e0f0f94962fc0a8c1a0f0527dc8e592c67939c46c903b6016cc0a8515da0044d"),),
			(3, hex_literal::hex!("2658c2083dcab9b118b5e828fb81344c4245deb8eed43fa890c8c0ae9cae526d"),),
			(3, hex_literal::hex!("123ca466ff6a76cdf3e73dc01bc1e8c4db195249e3b2e39a90036c6ded3db93a"),),
			(2, hex_literal::hex!("2eba9a39dbfdd5f3cba964355d45e27319f0271023c0353d97dc6df2401b0e3d"),),
			(2, hex_literal::hex!("b2636043fc3b8dfa608167a9fb6fb9d065b9f2f5821dc4bfc9785a244b24a92a"),),
			(2, hex_literal::hex!("9e6eb74b0a6b39de36fb58d1fab20bc2b3fea96023ce5a47941c20480d99f92e"),),
			(2, hex_literal::hex!("ee3d9d8c48ee88dce78fd7bafe3ce2052900eb465085b9324d4f5da26b145f2b"),),
			(2, hex_literal::hex!("d8290537d6e31fe1ff165eaa62b63f6f3556dcc720b0d3a6d7eab96275617304"),),
			(2, hex_literal::hex!("5a090c88f0438b46b451026597cee760a7bac9d396c9c7b529b68fb78aec5f43"),),
			(2, hex_literal::hex!("18d30040a8245c5ff17afc9a8169d7d0771fe7ab4135a64a022c254117340720"),),
			(1, hex_literal::hex!("b4f7f03bebc56ebe96bc52ea5ed3159d45a0ce3a8d7f082983c33ef133274747"),),
			(1, hex_literal::hex!("caafae0aaa6333fcf4dc193146945fe8e4da74aa6c16d481eef0ca35b8279d73"),),
			(1, hex_literal::hex!("e3d658975d1894d14c40bfa6f8b7e661cd2ee47b3f3c83f9258a4e9e8331df4e"),),
			(1, hex_literal::hex!("584d715bcb7a2d3b6a3120891dba91b19b12df42cd50f1c76103e2581d5b4274"),),
			(1, hex_literal::hex!("00ac81b86d05495a73dd7e98d33fb5bf55a837c6b87e3da0bf45618fed00be6d"),),
			(1, hex_literal::hex!("78e4813814891bd48bc745b79254a978833d41fbe0f387df93cd87eae2468926"),),
			(1, hex_literal::hex!("68170716ab7c6735dd0a1012045d9ea33891b5f6596cf97eb217d0962d86a518"),),
			(1, hex_literal::hex!("585e982d74da4f4290d20a73800cfd705cf59e1f5880aaee5506b5eaaf544f49"),),
			(1, hex_literal::hex!("d851f44a6f0d0d2f3439a51f2f75f66f4ea1a8e6c33c32f9af75fc188afb7546"),),
			(1, hex_literal::hex!("dca89b135d1a6aee0a498610a70eeaed056727c8a4d220da245842e540a54a74"),),
			(1, hex_literal::hex!("aa91fc0201f26b713a018669bcd269babf25368eee2493323b1ce0190a178a27"),),
			(1, hex_literal::hex!("dc20836f2e4b88c1858d1e3f918e7358043b4a8abcd2874e74d91d26c52eca2a"),),
			(1, hex_literal::hex!("9ebeef0150a33357023e678bfff549602e6943b5b85d8bfdb58473992fcfaf63"),),
			(1, hex_literal::hex!("307183930b2264c5165f4a210a99520c5f1672b0413d57769fabc19e6866fb25"),),
			(1, hex_literal::hex!("6201961514cf5ad87f1c4dd0c392ee28231f805f77975147bf2c33bd671b9822"),),
			(1, hex_literal::hex!("c6f57237cd4abfbeed99171495fc784e45a9d5d2814d435de40de00991a73c06"),),
			(1, hex_literal::hex!("c1df5c7e8ca56037450c58734326ebe34aec8f7d1928322a12164856365fea73"),),
			(1, hex_literal::hex!("12c039004da5e1e846aae808277098c719cef1f4985aed00161a42ac4f0e002f"),),
			(1, hex_literal::hex!("7460ac178015d2a7c289bb68ef9fdaac071596ab4425c276a0040aaac7055566"),),
			(1, hex_literal::hex!("eec4bd650a277342ebba0954ac786df2623bd6a9d6d3e69b484482336c549f79"),),
			(1, hex_literal::hex!("ca76c36de0085c8c561dbb64575cb016d4d6e7cef42b666d3ea978543f1c935a"),),
		];
	}

	/// Implements `OnRuntimeUpgrade` trait.
	pub struct Migration<T, I = ()>(PhantomData<(T, I)>);

	impl<T: Config<I>, I: 'static> OnRuntimeUpgrade for Migration<T, I>
	where
		<T as frame_system::Config>::AccountId: From<[u8; 32]>,
	{
		#[cfg(feature = "try-runtime")]
		fn pre_upgrade() -> Result<Vec<u8>, &'static str> {
			let onchain_version = Pallet::<T, I>::on_chain_storage_version();
			assert_eq!(onchain_version, 0, "the storage version must be 0.");
			let member_count = MemberCount::<T, I>::get(0);
			assert_eq!(member_count, 0, "the collective must be uninitialized.");

			Ok(())
		}

		fn on_runtime_upgrade() -> Weight {
			let current_version = RankedCollective::<T, I>::current_storage_version();
			let onchain_version = RankedCollective::<T, I>::on_chain_storage_version();
			let mut weight = T::DbWeight::get().reads(1);
			log::info!(
				target: TARGET,
				"running migration with current storage version {:?} / onchain {:?}.",
				current_version,
				onchain_version
			);
			if onchain_version != 0 {
				log::warn!(
					target: TARGET,
					"unsupported storage version, skipping import_fellowship migration."
				);
				return weight
			}
			let member_count = MemberCount::<T, I>::get(0);
			weight.saturating_accrue(T::DbWeight::get().reads(1));
			if member_count != 0 {
				log::warn!(
					target: TARGET,
					"the collective already initialized, skipping import_fellowship migration."
				);
				return weight
			}

			let mut max_rank = 0;
			for (rank, account_id32) in FellowshipAddresses::get() {
				let who: T::AccountId = account_id32.into();
				Members::<T, I>::insert(&who, MemberRecord::new(rank));
				weight.saturating_accrue(T::DbWeight::get().writes(1));
				for inner_rank in 0..rank + 1 {
					let index = MemberCount::<T, I>::get(rank);
					MemberCount::<T, I>::insert(inner_rank, index + 1);
					IdToIndex::<T, I>::insert(inner_rank, &who, index);
					IndexToId::<T, I>::insert(inner_rank, index, &who);
					max_rank = max_rank.max(inner_rank);
					// 2 writes to IdToIndex and IndexToId.
					weight.saturating_accrue(T::DbWeight::get().writes(2));
				}
			}
			// writes to MemberCount.
			weight.saturating_accrue(T::DbWeight::get().writes((max_rank as u64) + 1));

			weight
		}

		#[cfg(feature = "try-runtime")]
		fn post_upgrade(state: Vec<u8>) -> Result<(), &'static str> {
			assert_eq!(MemberCount::<T, I>::get(0), 45, "invalid members count at rank 0.");
			assert_eq!(MemberCount::<T, I>::get(1), 45, "invalid members count at rank 1.");
			assert_eq!(MemberCount::<T, I>::get(2), 24, "invalid members count at rank 2.");
			assert_eq!(MemberCount::<T, I>::get(3), 17, "invalid members count at rank 3.");
			assert_eq!(MemberCount::<T, I>::get(4), 10, "invalid members count at rank 4.");
			assert_eq!(MemberCount::<T, I>::get(5), 7, "invalid members count at rank 5.");
			assert_eq!(MemberCount::<T, I>::get(6), 3, "invalid members count at rank 6.");
			assert_eq!(MemberCount::<T, I>::get(7), 0, "invalid members count at rank 7.");
			Ok(())
		}
	}
}

#[cfg(test)]
pub mod tests {
	use super::import_kusama_fellowship::FellowshipAddresses;
	use pallet_ranked_collective::Rank;
	use parachains_common::AccountId;
	use sp_core::crypto::Ss58Codec;
	use sp_runtime::AccountId32;

	#[test]
	fn check_fellowship_addresses() {
		let fellowship_addresses = FellowshipAddresses::get();
		let kusama_fellowship_ss58: [(Rank, _); 45] = [
			(6, "FcxNWVy5RESDsErjwyZmPCW6Z8Y3fbfLzmou34YZTrbcraL"),
			(6, "J8ww78Qx3LVLW54bva3t4SzXcWKMdUWHEZR3V2VNKbmQgE8"),
			(6, "FFFF3gBSSDFSvK2HBq4qgLH75DHqXWPHeCnR1BSksAMacBs"),
			(5, "G7YVCdxZb8JLpAm9WMnJdNuojNT84AzU62zmvx5P1FMNtg2"),
			(5, "FczL661VujsMFqZT32Y3iBmNpSxUw4gMnq2JtdoFT6rJzr4"),
			(5, "Dcm1BqR4N7nHuV43TXdET7pNibt1Nzm42FggPHpxKRven53"),
			(5, "J9nD3s7zssCX7bion1xctAF6xcVexcpy2uwy4jTm9JL8yuK"),
			(4, "EGVQCe73TpFyAZx5uKfE1222XfkT3BSKozjgcqzLBnc5eYo"),
			(4, "HL8bEp8YicBdrUmJocCAWVLKUaR2dd1y6jnD934pbre3un1"),
			(4, "HyBryanRsB1GGKa9ZfqvRc3XpTDipYyRvxNNyZYfWFcenhd"),
			(3, "GAWwkmjbLhM5pnAVbdZEkwd3QjHE7kaxJSCi3Ec91Q3QSDW"),
			(3, "H25aCspunTUqAt4D1gC776vKZ8FX3MvQJ3Jde6qDXPQaFxk"),
			(3, "GtLQoW4ZqcjExMPq6qB22bYc6NaX1yMzRuGWpSRiHqnzRb9"),
			(3, "G5MVrgFmBaYei8N6t6DnDrb8JE53wKDkazLv5f46wVpi14y"),
			(3, "HfFpz4QUxfbocHudf8UU7cMgHqkHpf855Me5X846PZAsAYE"),
			(3, "DSbhnaGBytDGRfZTmdcArzCL6T3HQ8gcZxWpF5gLBP6y1Qe"),
			(3, "CzEPpMr7XNS6dK7nQFnQbfnJQYLq7nvULK5kL9U8Zb6CTJm"),
			(2, "Ddb9puChKMHq4gM6o47E551wAmaNeu6kHngX1jzNNqAw782"),
			(2, "GcDZZCVPwkPqoWxx8vfLb4Yfpz9yQ1f4XEyqngSH8ygsL9p"),
			(2, "GA3yPifemubFga7sTSFtLY2KFFiSRp6Bb8w31FS4xqgAvCz"),
			(2, "HxhDbS3grLurk1dhDgPiuDaRowHY1xHCU8Vu8on3fdg85tx"),
			(2, "HTk3eccL7WBkiyxz1gBcqQRghsJigoDMD7mnQaz1UAbMpQV"),
			(2, "EcNWrSPSDcVBRymwr26kk4JVFg92PdoU5Xwp87W2FgFSt9c"),
			(2, "D8sM6vKjWaeKy2zCPYWGkLLbWdUtWQrXBTQqr4dSYnVQo21"),
			(1, "GfbnnEgRU94n9ed4RFZ6Z9dBAWs5obykigJSwXKU9hsT2uU"),
			(1, "HA5NtttvyZsxo4wGxGoJJSMaWtdEFZAuGUMFHVWD7fgenPv"),
			(1, "Hj44XnjZui7SQ3A5eBMoJFa4H4nVhiyWnL2i2xw5f1YqzRX"),
			(1, "Ea6jhP5gF4r7NqhkEoAXJDgSgYpNQNaTYU6gPsrEGfctaKR"),
			(1, "CbCmCwFkfFkQo7bQtVczYg7sJ3oue6Ez2Z4RMGR8gi8deRk"),
			(1, "FJq9JpA9P7EXbmfsN9YiewJaDbQyL6vQyksGtJvzfbn6zf8"),
			(1, "EvoLanodoqDsgHb98Ymbu41uXXKfCPDKxeM6dXHyJ2JoVus"),
			(1, "EaBqDJJNsZmYdQ4xn1vomPJVNh7fjA6UztZeEjn7ZzdeT7V"),
			(1, "HTxCvXKVvUZ7PQq175kCRRLu7XkGfTfErrdNXr1ZuuwVZWv"),
			(1, "HZe91A6a1xqbKaw6ofx3GFepJjhVXHrwHEwn6YUDDFphpX9"),
			(1, "GRy2P3kBEzSHCbmDJfquku1cyUyhZaAqojRcNE4A4U3MnLd"),
			(1, "HYwiBo7Mcv7uUDg4MUoKm2fxzv4dMLAtmmNfzHV8qcQJpAE"),
			(1, "GAToWXwmQoMmxHKCmFJ615WbhdGRcRfyDZi7pg7PBRpQuNY"),
			(1, "DfqY6XQUSETTszBQ1juocTcG9iiDoXhvq1CoVadBSUqTGJS"),
			(1, "EnpgVWGGQVrFdSB2qeXRVdtccV6U5ZscNELBoERbkFD8Wi6"),
			(1, "H5BuqCmucJhUUuvjAzPazeVwVCtUSXVQdc5Dnx2q5zD7rVn"),
			(1, "GxX7S1pTDdeaGUjpEPPF2we6tgHDhbatFG25pVmVFtGHLH6"),
			(1, "CzuUtvKhZNZBjyAXeYviaRXwrLhVrsupJ9PrWmdq7BJTjGR"),
			(1, "FCunn2Rx8JqfT5g6noUKKazph4jLDba5rUee7o3ZmJ362Ju"),
			(1, "HyPMjWRHCpJS7x2SZ2R6M2XG5ZiCiZag4U4r7gBHRsE5mTc"),
			(1, "H9nUFL5DasuMeAiTC77QyZFCVX39crW6h7knXNrDF4PrSJf"),
		];

		for (index, val) in kusama_fellowship_ss58.iter().enumerate() {
			let account: AccountId32 = <AccountId as Ss58Codec>::from_string(val.1).unwrap();
			let account32: [u8; 32] = account.clone().into();
			assert_eq!(
				fellowship_addresses[index].0, kusama_fellowship_ss58[index].0,
				"ranks must be equal."
			);
			assert_eq!(fellowship_addresses[index].1, account32, "accounts must be equal.");
		}
	}
}
