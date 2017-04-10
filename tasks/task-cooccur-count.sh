#!/bin/bash
#PBS -l procs=1
# remember mem should be 8 * cores, for the whole job
CHUNK_SIZE=25
BASE="/scratch/sgalla19/cabarrus"
FILELIST="$BASE/all-xzs.lines"
ARCHIVES=`head -n $(($CHUNK_SIZE * $PBS_ARRAYID)) "$FILELIST" | tail -n $CHUNK_SIZE`
head -n $(($CHUNK_SIZE * $PBS_ARRAYID)) "$FILELIST" | tail -n $CHUNK_SIZE | xargs xzcat | $BASE/target/release/cb-cooccur $BASE/words $BASE/closed-cooccur/closed-ooccurs-${PBS_ARRAYID}.npy
#head -n $(($CHUNK_SIZE * $PBS_ARRAYID)) "$FILELIST" | tail -n $CHUNK_SIZE | xargs -- xz -t 
