//! This script uses the Sieve of Eratosthenes to calculate prime numbers.
const MAX_NUMBER_TO_CHECK = 1_000_000;
let prime_mask = new_array(MAX_NUMBER_TO_CHECK + 1, true);
prime_mask[0] = false;
prime_mask[1] = false;
let total_primes_found = 0;
for p in 2..=MAX_NUMBER_TO_CHECK {
    if !prime_mask[p] { continue; }
    total_primes_found += 1;
    let i = 2 * p;
    while i <= MAX_NUMBER_TO_CHECK {
        prime_mask[i] = false;
        i += p;
    }
}
total_primes_found