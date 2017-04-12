#!/bin/bash
#PBS -l procs=1
CHUNK_SIZE=25
ARCHIVES=`head -n $(($CHUNK_SIZE * $PBS_ARRAYID)) "$FILELIST" | tail -n $CHUNK_SIZE`
#echo "Would get file $ARCHIVE"
head -n $(($CHUNK_SIZE * $PBS_ARRAYID)) "$FILELIST" | tail -n $CHUNK_SIZE | xargs xzcat | /scratch/sgalla19/cabarrus/target/release/cb-token-count 
#head -n $(($CHUNK_SIZE * $PBS_ARRAYID)) "$FILELIST" | tail -n $CHUNK_SIZE | xargs -- xz -t 
