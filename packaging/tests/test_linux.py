import importlib.util,json,os,subprocess,tempfile,unittest
from pathlib import Path
from unittest.mock import patch
spec=importlib.util.spec_from_file_location('installer',Path(__file__).resolve().parents[1]/'linux/installer.py');m=importlib.util.module_from_spec(spec);spec.loader.exec_module(m)
class InstallTests(unittest.TestCase):
 def setUp(self):
  self.temp=tempfile.TemporaryDirectory();self.base=Path(self.temp.name);self.home=self.base/'home space';self.home.mkdir();self.bundle=self.base/'bundle';self.bundle.mkdir();self.env=patch.dict(os.environ,{'XDG_DATA_HOME':str(self.home/'.local/share')});self.env.start()
  payload={}
  for key in ['binary','svg','png-48','png-128','png-256','png-512']:
   raw=b'#!/bin/sh\nprintf "%s" "$1"\n' if key=='binary' else key.encode();p=self.bundle/key;p.write_bytes(raw);payload[key]={'path':key,'sha256':m.digest(raw)}
  self.manifest={'schema':1,'platform':'Linux','arch':'any-test','version':'test','products':[{'slug':'codecedar','name':'CodeCedar','available':True,'payload':payload},{'slug':'pycedar','name':'PyCedar','available':False}]};self.write();self.i=m.Installer(self.bundle,self.home)
 def tearDown(self):self.env.stop();self.temp.cleanup()
 def write(self):(self.bundle/'install-manifest.json').write_text(json.dumps(self.manifest))
 def test_install_menu_icons_launcher_and_verify(self):
  launcher=self.i.install('codecedar');self.assertTrue(self.i.verify());self.assertEqual(subprocess.check_output([launcher,'test arg']).decode(),'test arg');text=self.i.paths['desktop'].read_text();self.assertIn('Icon=codecedar',text);self.assertIn('Categories=Development;IDE;',text);self.assertIn('Exec="',text);self.assertEqual(len(self.i.old()),8)
 def test_uninstall_preserves_projects_and_preferences(self):
  self.i.install('codecedar');project=self.i.app/'my-project';project.write_text('mine');prefs=self.home/'.config/kedr/preferences.json';prefs.parent.mkdir(parents=True);prefs.write_text('{}');self.assertFalse(self.i.uninstall());self.assertEqual(project.read_text(),'mine');self.assertTrue(prefs.exists());self.assertFalse(self.i.paths['desktop'].exists())
 def test_upgrade_is_repeatable(self):
  self.i.install('codecedar');self.i.install('codecedar');self.assertTrue(self.i.verify())
 def test_unowned_file_is_never_overwritten(self):
  self.i.bin.mkdir(parents=True);self.i.paths['launcher'].write_text('mine');self.assertRaises(ValueError,self.i.install,'codecedar');self.assertEqual(self.i.paths['launcher'].read_text(),'mine');self.assertFalse(self.i.paths['binary'].exists())
 def test_modified_managed_file_blocks_upgrade(self):
  self.i.install('codecedar');self.i.paths['svg'].write_text('my custom icon');self.assertRaises(ValueError,self.i.install,'codecedar');self.assertEqual(self.i.paths['svg'].read_text(),'my custom icon')
 def test_modified_file_is_preserved_on_uninstall(self):
  self.i.install('codecedar');self.i.paths['svg'].write_text('mine');self.assertIn('svg',self.i.uninstall());self.assertEqual(self.i.paths['svg'].read_text(),'mine')
 def test_checksum_failure_has_no_side_effects(self):
  (self.bundle/'binary').write_text('corrupt');self.assertRaises(ValueError,self.i.install,'codecedar');self.assertFalse(self.i.state.exists());self.assertFalse(self.i.paths['binary'].exists())
 def test_planned_product_cannot_install(self):
  self.assertRaises(ValueError,self.i.install,'pycedar');self.assertFalse(self.i.state.exists())
 def test_unknown_product_cannot_install(self):self.assertRaises(ValueError,self.i.install,'unknown')
 def test_architecture_mismatch_is_rejected(self):
  self.manifest['arch']='not-this-machine';self.write();self.assertRaises(ValueError,m.Installer(self.bundle,self.home).install,'codecedar')
 def test_source_traversal_is_rejected(self):
  self.manifest['products'][0]['payload']['binary']['path']='../outside';self.write();self.assertRaises(ValueError,m.Installer(self.bundle,self.home).install,'codecedar')
 def test_source_symlink_is_rejected(self):
  (self.bundle/'binary').unlink();(self.bundle/'binary').symlink_to('/etc/passwd');self.assertRaises(ValueError,self.i.install,'codecedar')
 def test_destination_symlink_is_rejected(self):
  self.i.bin.mkdir(parents=True);outside=self.base/'outside';outside.write_text('mine');self.i.paths['launcher'].symlink_to(outside);self.assertRaises(ValueError,self.i.install,'codecedar');self.assertEqual(outside.read_text(),'mine')
 def test_bad_ownership_record_cannot_delete_outside_files(self):
  self.i.state.parent.mkdir(parents=True);self.i.state.write_text(json.dumps({'schema':1,'files':{'../../secret':'0'*64}}));self.assertRaises(ValueError,self.i.uninstall)
 def test_partial_write_failure_rolls_back(self):
  self.i.install('codecedar');before={k:p.read_bytes() for k,p in self.i.paths.items()};atomic=self.i.atomic;calls=[]
  def fail(path,data,mode):
   calls.append(path)
   if len(calls)==2:raise OSError('simulated disk failure')
   atomic(path,data,mode)
  with patch.object(self.i,'atomic',side_effect=fail):self.assertRaises(OSError,self.i.install,'codecedar')
  self.assertEqual(before,{k:p.read_bytes() for k,p in self.i.paths.items()});self.assertTrue(self.i.verify())
 def test_uninstall_preflight_rejects_symlink_before_deleting(self):
  self.i.install('codecedar');self.i.paths['svg'].unlink();self.i.paths['svg'].symlink_to('/etc/passwd');self.assertRaises(ValueError,self.i.uninstall);self.assertTrue(self.i.paths['binary'].exists())
 def test_relative_xdg_setting_is_ignored(self):
  with patch.dict(os.environ,{'XDG_DATA_HOME':'relative'}):self.assertEqual(m.Installer(self.bundle,self.home).data,self.home/'.local/share')
 def test_desktop_exec_and_launcher_escape_special_paths(self):
  home=self.base/'special $`%"\' space';home.mkdir()
  with patch.dict(os.environ,{'XDG_DATA_HOME':str(home/'share')}):
   i=m.Installer(self.bundle,home);launcher=i.install('codecedar');self.assertEqual(subprocess.check_output([launcher,'safe']).decode(),'safe');self.assertIn('%%',i.paths['desktop'].read_text());self.assertIn('\\$',i.paths['desktop'].read_text())
if __name__=='__main__':unittest.main()
