// Run the StorageBackend contract suite against the reference
// in-memory implementation. Any new test added to
// `storage-contract.ts` is automatically exercised here.

import { defineContractSuite } from './storage-contract';
import { MemoryStorage } from './storage-memory';

defineContractSuite('MemoryStorage', () => new MemoryStorage());
