#!/usr/bin/env python3
# Writes the manifests of the retry census: python3 make_manifests.py <dir>
import os, sys

CERT = {  # problem -> alternations (more for the short ones)
    'sys3x2_N3': 11, 'sys3x2_N4': 9, 'sys3x2_N5': 9, 'sys3x2_N6': 7, 'sys3x2_N7': 5,
    'masses2_N4': 9, 'masses2_N5': 7, 'masses2_N6': 7, 'masses2_N7': 5, 'masses2_N8': 5, 'masses2_N10': 5, 'masses2_N12': 5,
    'pop_006': 5, 'pop_001': 5, 'pop_019': 5,
}
NINE = ['sys3x2_N5', 'rand_t8_x5_m20_s0', 'rand_t12_x5_m20_s0', 'pop_017', 'pop_005', 'masses3_N4', 'masses4_N3', 'masses5_N3', 'pop_002']
FIFTY_ONE = ('di_doc_N3 di_N10 di_N12 di_N15 di_N20 di_N25 di_N30 di_N4 di_N6 di_N8 dinv_N4 dinv_N6 dinv_N8 masses2_N4 masses2_N5 masses2_N6 '
             'masses2_N7 masses2_N8 masses3_N3 masses3_N4 masses4_N3 masses4_N4 masses5_N2 masses5_N3 masses6_N2 masses7_N2 masses8_N2 pop_005 '
             'pop_017 pop_021 pop_027 rand_t10_x5_m15_s0 rand_t10_x5_m20_s0 rand_t12_x5_m15_s0 rand_t12_x5_m20_s0 rand_t14_x5_m15_s0 '
             'rand_t16_x5_m15_s0 rand_t3_x5_m20_s0 rand_t4_x5_m20_s0 rand_t4_x5_m20_s1 rand_t5_x5_m20_s0 rand_t5_x5_m20_s1 rand_t5_x5_m20_s2 '
             'rand_t5_x6_m24_s0 rand_t6_x5_m20_s0 rand_t8_x5_m20_s0 sys3x2_N3 sys3x2_N4 sys3x2_N5 sys3x2_N6 sys3x2_N7').split()


def main():
    d = sys.argv[1]
    os.makedirs(d, exist_ok=True)
    assert len(FIFTY_ONE) == 51
    w = lambda n, lines: open(os.path.join(d, n), 'w').write('\n'.join(lines) + '\n')
    w('cert.txt', ['cert %s %d' % (p, a) for p, n in CERT.items() for a in range(n)])
    w('base.txt', ['base %s %d' % (p, a) for p in FIFTY_ONE for a in range(3)])
    w('c23.txt', ['c23 %s' % p for p in NINE if p != 'pop_002'])
    w('c23_big.txt', ['c23 pop_002'])
    w('c4.txt', ['c4 %s' % p for p in FIFTY_ONE])
    w('problems.txt', sorted(set(CERT) | set(NINE) | set(FIFTY_ONE)))


main()
