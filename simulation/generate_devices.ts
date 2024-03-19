import { appendFile } from 'fs/promises';
import { randomBytes } from 'crypto'

(async () => {
    for(let i = 0; i < 10000; i++) {
        let dev_eui = randomBytes(8);
        let join_eui = randomBytes(8);
        let key = randomBytes(16);
        let new_line = `${dev_eui.toString('hex')},${join_eui.toString('hex')},${key.toString('hex')}\n`;
        const f = await appendFile('./devices_augmented.csv', new_line);
    }
})()