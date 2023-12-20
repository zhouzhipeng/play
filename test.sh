#!/bin/bash

aa(){
  cd server
  export AA=1
  echo $(pwd)
  cd ..
}

bb(){
  aa;
  echo $(pwd)
  echo $AA
}

bb;