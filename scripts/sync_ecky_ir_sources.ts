import { syncEckyIrContent } from './ecky_ir_content';
import { syncSplitBook } from './ecky_ir_source';

const check = process.argv.includes('--check');
syncEckyIrContent(process.cwd(), check);
syncSplitBook(process.cwd(), check);
console.log(check ? 'Ecky content sources are current.' : 'Ecky content sources synchronized.');
