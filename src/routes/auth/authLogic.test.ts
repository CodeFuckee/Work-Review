import test from 'node:test';
import assert from 'node:assert/strict';

import {
  findLoginValidationError,
  findRegisterValidationError,
  parseTokenExpiresAt,
  resolveAuthGate,
} from './authLogic.ts';

function makeJwt(payload: Record<string, unknown>): string {
  const encode = (value: unknown) =>
    Buffer.from(JSON.stringify(value)).toString('base64url');
  return `${encode({ alg: 'HS256', typ: 'JWT' })}.${encode(payload)}.signature`;
}

test('parseTokenExpiresAt 从 JWT payload 中解析 exp', () => {
  const token = makeJwt({ sub: '42', exp: 1_758_000_000 });
  assert.equal(parseTokenExpiresAt(token), 1_758_000_000);
});

test('parseTokenExpiresAt 支持 base64url 字符与 UTF-8 载荷', () => {
  const token = makeJwt({ name: '张三', exp: 123 });
  assert.equal(parseTokenExpiresAt(token), 123);
});

test('parseTokenExpiresAt 对非法输入返回 null', () => {
  assert.equal(parseTokenExpiresAt('not-a-jwt'), null);
  assert.equal(parseTokenExpiresAt('a.b.c'), null);
  assert.equal(parseTokenExpiresAt(makeJwt({ sub: 'no-exp' })), null);
});

test('resolveAuthGate：未登录强制登录，过期离线宽限，其余放行', () => {
  assert.equal(resolveAuthGate(false, false), 'login');
  assert.equal(resolveAuthGate(false, true), 'login');
  assert.equal(resolveAuthGate(true, false), 'allow');
  assert.equal(resolveAuthGate(true, true), 'grace');
});

test('findLoginValidationError：账号或密码为空时报错', () => {
  assert.equal(findLoginValidationError('', 'pw'), 'auth.errorAccountRequired');
  assert.equal(findLoginValidationError('  ', 'pw'), 'auth.errorAccountRequired');
  assert.equal(findLoginValidationError('alice', ''), 'auth.errorAccountRequired');
  assert.equal(findLoginValidationError('alice', 'pw'), null);
});

test('findRegisterValidationError：逐项校验注册表单', () => {
  assert.equal(
    findRegisterValidationError({ userName: '', password: '123456', confirmPassword: '123456', name: '张三' }),
    'auth.errorUserNameRequired',
  );
  assert.equal(
    findRegisterValidationError({ userName: 'alice', password: '123456', confirmPassword: '123456', name: ' ' }),
    'auth.errorNameRequired',
  );
  assert.equal(
    findRegisterValidationError({ userName: 'alice', password: '12345', confirmPassword: '12345', name: '张三' }),
    'auth.errorPasswordTooShort',
  );
  assert.equal(
    findRegisterValidationError({ userName: 'alice', password: '123456', confirmPassword: '654321', name: '张三' }),
    'auth.errorPasswordMismatch',
  );
  assert.equal(
    findRegisterValidationError({ userName: ' alice ', password: '123456', confirmPassword: '123456', name: '张三' }),
    null,
  );
});
