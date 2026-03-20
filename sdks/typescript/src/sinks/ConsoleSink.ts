import { once } from 'node:events';

import type { Sink } from './Sink.js';
import type { WideEventPayload } from '../types.js';

export class ConsoleSink implements Sink {
  async export(event: WideEventPayload): Promise<void> {
    if (!process.stdout.write(`${JSON.stringify(event)}\n`)) {
      await once(process.stdout, 'drain');
    }
  }
}
