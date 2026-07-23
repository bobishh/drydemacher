import { syncSplitBook } from './ecky_ir_source';

const check = process.argv.includes('--check');
syncSplitBook(process.cwd(), check);
console.log(check ? 'Split book source is current.' : 'Split book source synchronized.');
