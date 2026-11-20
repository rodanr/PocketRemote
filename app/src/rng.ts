// Cryptographically secure randomness from the OS via expo-crypto. Never Math.random.
import { getRandomBytes } from 'expo-crypto';
import type { Rng } from './noise';

export const secureRng: Rng = (n) => getRandomBytes(n);
