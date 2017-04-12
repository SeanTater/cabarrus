#!/bin/bash
#PBS -l procs=1
#PBS -l mem=40GB
# the glove corpus needs about 40GB per core
CHUNK_SIZE=25
BASE="/scratch/sgalla19/cabarrus"
FILELIST="$BASE/all-xzs.lines"
ARCHIVES=`head -n $(($CHUNK_SIZE * $PBS_ARRAYID)) "$FILELIST" | tail -n $CHUNK_SIZE`
head -n $(($CHUNK_SIZE * $PBS_ARRAYID)) "$FILELIST" | tail -n $CHUNK_SIZE | xargs xzcat | $BASE/target/release/cb-approx-cooccur $BASE/allglove-uncased.lines $BASE/approx-cooccur/approx-cooccur-${PBS_ARRAYID}.npy
