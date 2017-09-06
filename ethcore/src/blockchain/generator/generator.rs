// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

use bigint::prelude::U256;
use bigint::hash::H2048;
use bytes::Bytes;
use header::BlockNumber;
use transaction::SignedTransaction;
use super::fork::Fork;
use super::bloom::Bloom;
use super::complete::{BlockFinalizer, CompleteBlock, Complete};
use super::block::Block;
use super::transaction::Transaction;

/// Chain iterator interface.
pub trait ChainIterator: Iterator + Sized {
	/// Should be called to create a fork of current iterator.
	/// Blocks generated by fork will have lower difficulty than current chain.
	fn fork(&self, fork_number: usize) -> Fork<Self> where Self: Clone;
	/// Should be called to make every consecutive block have given bloom.
	fn with_bloom(&mut self, bloom: H2048) -> Bloom<Self>;
	/// Should be called to make every consecutive block have given transaction.
	fn with_transaction(&mut self, transaction: SignedTransaction) -> Transaction<Self>;
	/// Should be called to complete block. Without complete, block may have incorrect hash.
	fn complete<'a>(&'a mut self, finalizer: &'a mut BlockFinalizer) -> Complete<'a, Self>;
	/// Completes and generates block.
	fn generate<'a>(&'a mut self, finalizer: &'a mut BlockFinalizer) -> Option<Bytes> where Self::Item: CompleteBlock;
}

impl<I> ChainIterator for I where I: Iterator + Sized {
	fn fork(&self, fork_number: usize) -> Fork<Self> where I: Clone {
		Fork {
			iter: self.clone(),
			fork_number: fork_number
		}
	}

	fn with_bloom(&mut self, bloom: H2048) -> Bloom<Self> {
		Bloom {
			iter: self,
			bloom: bloom
		}
	}

	fn with_transaction(&mut self, transaction: SignedTransaction) -> Transaction<Self> {
		Transaction {
			iter: self,
			transaction: transaction,
		}
	}

	fn complete<'a>(&'a mut self, finalizer: &'a mut BlockFinalizer) -> Complete<'a, Self> {
		Complete {
			iter: self,
			finalizer: finalizer
		}
	}

	fn generate<'a>(&'a mut self, finalizer: &'a mut BlockFinalizer) -> Option<Bytes> where <I as Iterator>::Item: CompleteBlock {
		self.complete(finalizer).next()
	}
}

/// Blockchain generator.
#[derive(Clone)]
pub struct ChainGenerator {
	/// Next block number.
	number: BlockNumber,
	/// Next block difficulty.
	difficulty: U256,
}

impl ChainGenerator {
	fn prepare_block(&self) -> Block {
		let mut block = Block::default();
		block.header.set_number(self.number);
		block.header.set_difficulty(self.difficulty);
		block
	}
}

impl Default for ChainGenerator {
	fn default() -> Self {
		ChainGenerator {
			number: 0,
			difficulty: 1000.into(),
		}
	}
}

impl Iterator for ChainGenerator {
	type Item = Block;

	fn next(&mut self) -> Option<Self::Item> {
		let block = self.prepare_block();
		self.number += 1;
		Some(block)
	}
}

mod tests {
	use bigint::hash::{H256, H2048};
	use views::BlockView;
	use blockchain::generator::{ChainIterator, ChainGenerator, BlockFinalizer};

	#[test]
	fn canon_chain_generator() {
		let mut canon_chain = ChainGenerator::default();
		let mut finalizer = BlockFinalizer::default();

		let genesis_rlp = canon_chain.generate(&mut finalizer).unwrap();
		let genesis = BlockView::new(&genesis_rlp);

		assert_eq!(genesis.header_view().parent_hash(), H256::default());
		assert_eq!(genesis.header_view().number(), 0);

		let b1_rlp = canon_chain.generate(&mut finalizer).unwrap();
		let b1 = BlockView::new(&b1_rlp);

		assert_eq!(b1.header_view().parent_hash(), genesis.header_view().hash());
		assert_eq!(b1.header_view().number(), 1);

		let mut fork_chain = canon_chain.fork(1);

		let b2_rlp_fork = fork_chain.generate(&mut finalizer.fork()).unwrap();
		let b2_fork = BlockView::new(&b2_rlp_fork);

		assert_eq!(b2_fork.header_view().parent_hash(), b1.header_view().hash());
		assert_eq!(b2_fork.header_view().number(), 2);

		let b2_rlp = canon_chain.generate(&mut finalizer).unwrap();
		let b2 = BlockView::new(&b2_rlp);

		assert_eq!(b2.header_view().parent_hash(), b1.header_view().hash());
		assert_eq!(b2.header_view().number(), 2);
		assert!(b2.header_view().difficulty() > b2_fork.header_view().difficulty());
	}

	#[test]
	fn with_bloom_generator() {
		let bloom = H2048([0x1; 256]);
		let mut gen = ChainGenerator::default();
		let mut finalizer = BlockFinalizer::default();

		let block0_rlp = gen.with_bloom(bloom).generate(&mut finalizer).unwrap();
		let block1_rlp = gen.generate(&mut finalizer).unwrap();
		let block0 = BlockView::new(&block0_rlp);
		let block1 = BlockView::new(&block1_rlp);

		assert_eq!(block0.header_view().number(), 0);
		assert_eq!(block0.header_view().parent_hash(), H256::default());

		assert_eq!(block1.header_view().number(), 1);
		assert_eq!(block1.header_view().parent_hash(), block0.header_view().hash());

	}

	#[test]
	fn generate_1000_blocks() {
		let generator = ChainGenerator::default();
		let mut finalizer = BlockFinalizer::default();
		let blocks: Vec<_> = generator.take(1000).complete(&mut finalizer).collect();
		assert_eq!(blocks.len(), 1000);
	}
}

