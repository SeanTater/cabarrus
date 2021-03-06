import numpy as np
import sqlite3
unif = np.random.uniform
vocab = 10000

def zero_is_one(a):
    a[a==0]=1
    return a
noop = lambda x: x
l1_norm = lambda x: x / zero_is_one(x.sum(axis=1).reshape(-1, 1))
l2_norm = lambda x: x / zero_is_one(np.sqrt((x*x).sum(axis=1)).reshape((-1, 1)))
bins_threshold = lambda x, keep: np.sign(x) * (np.abs(x) < keep)

approaches = {
    "uniform": lambda factors: np.random.uniform(-1.0, 1.0, (vocab, factors)),
    "normal": lambda factors: np.random.normal(0.0, 1.0, (vocab, factors)),
    "bins_threshold_0.05": lambda factors: bins_threshold(unif(-1.0, 1.0, (vocab, factors)), 0.05),
    "bins_threshold_0.1": lambda factors: bins_threshold(unif(-1.0, 1.0, (vocab, factors)), 0.1),
    "bins_threshold_0.2": lambda factors: bins_threshold(unif(-1.0, 1.0, (vocab, factors)), 0.2),
    "bins_threshold_0.4": lambda factors: bins_threshold(unif(-1.0, 1.0, (vocab, factors)), 0.4),
    "bins_threshold_0.8": lambda factors: bins_threshold(unif(-1.0, 1.0, (vocab, factors)), 0.8)
}

norms = {
    "l1": l1_norm,
    "l2": l2_norm,
    "noop": noop
}

def cossims(ins):
    mid = l2_norm(ins)
    return mid.dot(mid.T)

conn = sqlite3.connect("random_indexing.db")
conn.execute("CREATE TABLE IF NOT EXISTS rindex(approach text, norm text, fill real, factors real, mean real, stdev real)")

for apname, apfn in approaches.items():         # 7
    for nname, nfn in norms.items():            # 3
        for fill_exp in range(10):              # 10
            fill = 0.001 * (1<<fill_exp)
            for factors_exp in range(15):       # 15
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
                conn.execute("INSERT INTO rindex(approach, norm, fill, factors, mean, stdev) VALUES (?,?,?,?,?,?);",
                    [apname, nname, fill, factors, rmses.mean(), np.std(rmses)])
                conn.commit()