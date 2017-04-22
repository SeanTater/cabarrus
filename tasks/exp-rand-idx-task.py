#!/usr/bin/python
import numpy as np
import scipy as sp
import scipy.linalg
import sqlite3
import os
unif = np.random.uniform
vocab = 10000

def zero_is_one(a):
    a[a==0]=1
    return a

l1_norm = lambda x: x / zero_is_one(x.sum(axis=1).reshape(-1, 1))
l2_norm = lambda x: x / zero_is_one(np.sqrt((x*x).sum(axis=1)).reshape((-1, 1)))
bins_threshold = lambda x, keep: np.sign(x) * (np.abs(x) < keep)

approaches = sorted({
    "uniform": lambda factors: np.random.uniform(-1.0, 1.0, (vocab, factors)),
    "normal": lambda factors: np.random.normal(0.0, 1.0, (vocab, factors)),
    "bins_threshold_0.05": lambda factors: bins_threshold(unif(-1.0, 1.0, (vocab, factors)), 0.05),
    "bins_threshold_0.1": lambda factors: bins_threshold(unif(-1.0, 1.0, (vocab, factors)), 0.1),
    "bins_threshold_0.2": lambda factors: bins_threshold(unif(-1.0, 1.0, (vocab, factors)), 0.2),
    "bins_threshold_0.4": lambda factors: bins_threshold(unif(-1.0, 1.0, (vocab, factors)), 0.4),
    "bins_threshold_0.8": lambda factors: bins_threshold(unif(-1.0, 1.0, (vocab, factors)), 0.8)
}.items())

norms = sorted({
    "l1": l1_norm,
    "l2": l2_norm,
    "qr": lambda a: sp.linalg.qr(a, overwrite_a=True, mode='economic')[0],
    "noop": lambda x: x
}.items())


jobs = [
(apname, apfn, nname, nfn, fill_exp, factors_exp)
for apname, apfn in approaches                  # 7
    for nname, nfn in norms                     # 3
        for fill_exp in range(10)               # 10
            for factors_exp in range(15)        # 15
]


start_jobid = os.getenv("PBS_ARRAYID")
for jobid in range(start_jobid, start_jobid+10):
    print("# apname apfn nname nfn fill_exp factors_exp")
    print("#" + str(jobs[int(jobid)]))
    (apname, apfn, nname, nfn, fill_exp, factors_exp) = jobs[int(jobid)]

    fill = 0.001 * (1<<fill_exp)
    factors = 1<<factors_exp
    rmses = []
    for iteration in range(10):        # 10
        contexts = nfn(apfn(factors))

        # This is a binary field. Check more distributions.
        corefs = 1.0 * (unif(0.0, 1.0, (vocab, vocab)) < fill)
        #import code
        #code.interact(local=vars())
        transformed = corefs.T.dot(contexts)
        reconstructed = transformed.dot(contexts.T).T

        corefs /= corefs.sum()
        reconstructed /= reconstructed.sum()

        corefs -= reconstructed
        np.abs(corefs, corefs)

        rmses.append(corefs.sum() / (vocab*vocab*fill))
    rmses = np.array(rmses)
    print(",".join(str(x) for x in [apname, nname, fill, factors, rmses.mean(), np.std(rmses)]) )
