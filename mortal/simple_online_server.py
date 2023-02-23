#!/usr/bin/env python
# coding=utf-8

import prelude

import os
import sys
import json
import torch
from datetime import datetime, timezone
from model import Brain, DQN, GRP
from engine import MortalEngine
from common import filtered_trimmed_lines
from libriichi.mjai import Bot
from libriichi.dataset import Grp
from config import config

import pickle
from xmlrpc.server import SimpleXMLRPCServer

engine = None
bot = None

def bot_call(log_seq):
    resp = None
    try:
        print (log_seq)
        for log in log_seq:
            if log['type'] == 'start_game':
                global engine, bot
                bot = Bot(engine, log['player_id'])
        for log in log_seq[:-1]:
            bot.react(json.dumps(log))
        resp = bot.react(json.dumps(log_seq[-1]))
        print(resp)
    except Exception as e:
        print (e)
        raise e
    return pickle.dumps(resp)

def RPC_init():
    server = SimpleXMLRPCServer(('0.0.0.0',8081))
    server.register_function(bot_call, "bot_call")
    while True:
        server.handle_request()

def main():
    review_mode = os.environ.get('MORTAL_REVIEW_MODE', '0') == '1'

    device = torch.device('cpu')
    state = torch.load(config['control']['state_file'], map_location=torch.device('cpu'))
    cfg = state['config']
    version = cfg['control'].get('version', 1)
    num_blocks = cfg['resnet']['num_blocks']
    conv_channels = cfg['resnet']['conv_channels']
    time = datetime.fromtimestamp(state['timestamp'], tz=timezone.utc).strftime('%y%m%d%H')
    tag = f'mortal{version}-b{num_blocks}c{conv_channels}-t{time}'

    mortal = Brain(version=version, num_blocks=num_blocks, conv_channels=conv_channels).eval()
    dqn = DQN(version=version).eval()
    mortal.load_state_dict(state['mortal'])
    dqn.load_state_dict(state['current_dqn'])
    mortal.to(device)
    dqn.to(device)
    global engine
    engine = MortalEngine(
        mortal,
        dqn,
        version = version,
        is_oracle = False,
        device = device,
        enable_amp = False,
        enable_quick_eval = not review_mode,
        enable_rule_based_agari_guard = True,
        name = 'mortal',
    )
    RPC_init()

if __name__ == '__main__':
    try:
        main()
    except KeyboardInterrupt:
        pass
